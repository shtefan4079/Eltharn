pub use crate::requirement::Requirement;
use el_extension_api::{ExtensionId, services::RuntimeSide};
use el_launcher_core::extensions::PackagePin;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub id: ExtensionId,
    pub requirement: Requirement,
    #[serde(default)]
    pub optional: bool,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Conflict {
    pub id: ExtensionId,
    pub requirement: Requirement,
    pub reason: String,
}
#[derive(Clone, Debug)]
pub struct ResolvePackage {
    pub pin: PackagePin,
    pub modules: BTreeSet<String>,
    pub dependencies: Vec<Dependency>,
    pub conflicts: Vec<Conflict>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ActorSpec {
    pub package: PackagePin,
    pub slot: String,
    pub rank: usize,
    pub required_dependencies: Vec<usize>,
}
#[derive(Clone, Debug)]
pub struct ResolvedRoster {
    pub actors: Vec<ActorSpec>,
    pub disabled_optional: Vec<String>,
}
fn id_inner(id: &ExtensionId) -> String {
    id.to_string()
}
fn reason_ok(r: &str) -> bool {
    let n = r.chars().count();
    n >= 1 && n <= 256 && !r.chars().any(|c| c.is_control())
}
pub fn resolve(selected: &[ResolvePackage], side: RuntimeSide) -> Result<ResolvedRoster, String> {
    if selected.len() > 256 {
        return Err(format!(
            "too many packages: {} > 256 before init",
            selected.len()
        ));
    }
    let is_client = matches!(side, RuntimeSide::Client);
    let mut all_idx: BTreeMap<String, usize> = BTreeMap::new();
    for (i, p) in selected.iter().enumerate() {
        let t = id_inner(&p.pin.id);
        if let Some(_prev) = all_idx.insert(t.clone(), i) {
            return Err(format!("duplicate package id `{t}` before init"));
        }
        if p.pin.id.is_reserved_namespace() {
            return Err(format!("reserved package id `{t}` before init"));
        }
        if p.modules.is_empty() {
            return Err(format!("package `{t}` has no modules before init"));
        }
        for m in &p.modules {
            if m != "client" && m != "server" && m != "shared" {
                return Err(format!(
                    "package `{t}` has unknown module `{m}` before init"
                ));
            }
        }
    }
    for p in selected {
        let t = id_inner(&p.pin.id);
        if p.dependencies.len() > 256 {
            return Err(format!(
                "package `{t}` has too many dependencies before init"
            ));
        }
        if p.conflicts.len() > 256 {
            return Err(format!("package `{t}` has too many conflicts before init"));
        }
        let mut seen = BTreeSet::new();
        for d in &p.dependencies {
            let di = id_inner(&d.id);
            if !seen.insert(di.clone()) {
                return Err(format!(
                    "package `{t}` duplicates dependency `{di}` before init"
                ));
            }
            if di == t {
                return Err(format!("package `{t}` depends on itself before init"));
            }
        }
        let mut seen_c = BTreeSet::new();
        for c in &p.conflicts {
            let ci = id_inner(&c.id);
            if !seen_c.insert(ci.clone()) {
                return Err(format!(
                    "package `{t}` duplicates conflict `{ci}` before init"
                ));
            }
            if ci == t {
                return Err(format!("package `{t}` conflicts with itself before init"));
            }
            if !reason_ok(&c.reason) {
                return Err(format!(
                    "package `{t}` has invalid conflict reason for `{ci}` before init"
                ));
            }
        }
    }
    let active = |p: &ResolvePackage| {
        p.modules.iter().any(|m| {
            if is_client {
                m == "client" || m == "shared"
            } else {
                m == "server" || m == "shared"
            }
        })
    };
    let mut active_idx: BTreeMap<String, usize> = BTreeMap::new();
    for (t, &i) in &all_idx {
        if active(&selected[i]) {
            active_idx.insert(t.clone(), i);
        }
    }
    let mut adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut indeg: BTreeMap<String, usize> = BTreeMap::new();
    let mut mandatory: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut disabled: Vec<String> = Vec::new();
    for t in active_idx.keys() {
        indeg.insert(t.clone(), 0);
        adj.entry(t.clone()).or_default();
        mandatory.insert(t.clone(), Vec::new());
    }
    for (t, &i) in &active_idx {
        let p = &selected[i];
        for d in &p.dependencies {
            let di = id_inner(&d.id);
            let target = match all_idx.get(&di).map(|&j| (j, active_idx.contains_key(&di))) {
                None => None,
                Some((j, is_act)) => Some((j, is_act)),
            };
            let (j, is_act) = match target {
                None => {
                    if d.optional {
                        disabled.push(format!("{t}: optional dependency {di} disabled (missing)"));
                        continue;
                    }
                    return Err(format!(
                        "package `{t}` requires `{di}`: missing before init"
                    ));
                }
                Some(v) => v,
            };
            if !is_act {
                if d.optional {
                    disabled.push(format!(
                        "{t}: optional dependency {di} disabled (no active module)"
                    ));
                    continue;
                }
                return Err(format!(
                    "package `{t}` requires `{di}`: no active module before init"
                ));
            }
            let tp = &selected[j];
            if !d.requirement.matches(&tp.pin.version) {
                if d.optional {
                    disabled.push(format!(
                        "{t}: optional dependency {di} disabled (version mismatch)"
                    ));
                    continue;
                }
                return Err(format!(
                    "package `{t}` requires `{di}`: wrong version before init"
                ));
            }
            adj.entry(di.clone()).or_default().insert(t.clone());
            *indeg.get_mut(t).unwrap() += 1;
            if !d.optional {
                mandatory.get_mut(t).unwrap().push(di);
            }
        }
    }
    for (t, &i) in &active_idx {
        for c in &selected[i].conflicts {
            let ci = id_inner(&c.id);
            if let Some(&j) = active_idx.get(&ci) {
                if c.requirement.matches(&selected[j].pin.version) {
                    return Err(format!(
                        "conflict between `{t}` and `{ci}` before init: {}",
                        c.reason
                    ));
                }
            }
        }
    }
    let mut zero: BTreeSet<String> = indeg
        .iter()
        .filter(|(_, v)| **v == 0)
        .map(|(k, _)| k.clone())
        .collect();
    let mut order: Vec<String> = Vec::new();
    while let Some(n) = zero.iter().next().cloned() {
        zero.remove(&n);
        order.push(n.clone());
        if let Some(nexts) = adj.get(&n).cloned() {
            for m in nexts {
                let e = indeg.get_mut(&m).unwrap();
                *e -= 1;
                if *e == 0 {
                    zero.insert(m);
                }
            }
        }
    }
    if order.len() != active_idx.len() {
        let rest: Vec<String> = indeg
            .into_iter()
            .filter(|(_, v)| *v > 0)
            .map(|(k, _)| k)
            .collect();
        return Err(format!("dependency cycle before init: {}", rest.join(",")));
    }
    let slot_ok = |m: &str| {
        if is_client {
            m == "client" || m == "shared"
        } else {
            m == "server" || m == "shared"
        }
    };
    let mut pkg_ranks: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    let mut actors: Vec<ActorSpec> = Vec::new();
    for t in &order {
        let p = &selected[active_idx[t]];
        let slots: Vec<String> = p.modules.iter().filter(|m| slot_ok(m)).cloned().collect();
        let mut ranks = Vec::new();
        for s in slots {
            let rank = actors.len();
            ranks.push(rank);
            actors.push(ActorSpec {
                package: p.pin.clone(),
                slot: s,
                rank,
                required_dependencies: Vec::new(),
            });
        }
        pkg_ranks.insert(t.clone(), ranks);
    }
    for a in actors.iter_mut() {
        let owner = id_inner(&a.package.id);
        let mut reqs: Vec<usize> = Vec::new();
        if let Some(deps) = mandatory.get(&owner) {
            for d in deps {
                if let Some(rs) = pkg_ranks.get(d) {
                    reqs.extend(rs.iter().cloned());
                }
            }
        }
        reqs.sort_unstable();
        reqs.dedup();
        a.required_dependencies = reqs;
    }
    disabled.sort();
    disabled.dedup();
    Ok(ResolvedRoster {
        actors,
        disabled_optional: disabled,
    })
}
