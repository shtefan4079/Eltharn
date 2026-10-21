use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::spec::{Components, ContentSpec, FieldType, InventoryStack, Kind};
use crate::{ContentId, Definition, RegistryBuilder, StateSchema};
use el_extension_api::ExtensionId;

const MAX_ENTRIES: usize = 4096;
const MAX_PER_OWNER: usize = 512;
const MAX_BYTES: usize = 4 * 1024 * 1024;
const MAX_DEF_BYTES: usize = 64 * 1024;
const MAX_TOTAL_STATES: usize = 65536;
const MAX_TAG_RESOLVED: usize = 4096;
const MAX_TAG_DEPTH: usize = 64;
const MAX_COND_DEPTH: usize = 32;

const KINDS: [Kind; 9] = [
    Kind::Block,
    Kind::Item,
    Kind::Entity,
    Kind::BlockEntity,
    Kind::Component,
    Kind::Tag,
    Kind::Recipe,
    Kind::Loot,
    Kind::Condition,
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedSpec {
    pub owner: ExtensionId,
    pub spec: ContentSpec,
}

#[derive(Clone, Default, Debug)]
pub struct CatalogBuilder {
    entries: BTreeMap<(Kind, ContentId), OwnedSpec>,
    bytes: usize,
}

#[derive(Clone, Debug)]
pub struct Catalog {
    entries: BTreeMap<(Kind, ContentId), OwnedSpec>,
    fingerprint: String,
    resolved_tags: BTreeMap<ContentId, BTreeSet<ContentId>>,
    schemas: BTreeMap<ContentId, StateSchema>,
}

fn id_namespace(id: &ContentId) -> Result<String, String> {
    let s = id.to_string();
    s.split_once(':')
        .map(|(ns, _)| ns.to_string())
        .ok_or_else(|| format!("bad id (need ns:path): {s}"))
}

fn reserved_ns(ns: &str) -> bool {
    ns == "minecraft" || ns == "eltharn"
}

fn canonical_bytes(spec: &ContentSpec) -> Result<Vec<u8>, String> {
    let b = serde_json::to_vec(spec).map_err(|e| e.to_string())?;
    if b.len() > MAX_DEF_BYTES {
        return Err(format!("definition too large: {}", b.len()));
    }
    Ok(Definition::parse(&b)?.canonical_bytes().to_vec())
}

fn lookup<'a>(
    m: &'a BTreeMap<(Kind, ContentId), OwnedSpec>,
    kind: Kind,
    id: &ContentId,
) -> Result<&'a OwnedSpec, String> {
    m.get(&(kind, id.clone()))
        .ok_or_else(|| format!("missing ref {}:{}", kind.as_str(), id))
}

impl CatalogBuilder {
    pub fn register_batch(
        &mut self,
        owner: ExtensionId,
        batch: Vec<ContentSpec>,
    ) -> Result<(), String> {
        if owner.is_reserved_namespace() {
            return Err("reserved owner namespace".into());
        }
        if batch.is_empty() {
            return Ok(());
        }
        if self.entries.len() + batch.len() > MAX_ENTRIES {
            return Err("catalog entry limit exceeded".into());
        }
        let os = owner.to_string();
        let cur = self
            .entries
            .values()
            .filter(|e| e.owner.to_string() == os)
            .count();
        if cur + batch.len() > MAX_PER_OWNER {
            return Err("per-owner entry limit exceeded".into());
        }
        let mut seen: BTreeSet<(Kind, ContentId)> = BTreeSet::new();
        let mut staged: Vec<((Kind, ContentId), OwnedSpec)> = Vec::with_capacity(batch.len());
        let mut add = 0usize;
        for spec in &batch {
            let kind = spec.kind();
            let id = spec.id().clone();
            let ns = id_namespace(&id)?;
            if ns != owner.namespace() {
                return Err(format!("id {id} not namespaced by owner {os}"));
            }
            if reserved_ns(&ns) {
                return Err(format!("reserved namespace: {ns}"));
            }
            spec.validate_local()?;
            let key = (kind, id);
            if self.entries.contains_key(&key) {
                return Err("duplicate (kind,id)".into());
            }
            if !seen.insert(key.clone()) {
                return Err("duplicate (kind,id) within batch".into());
            }
            let b = canonical_bytes(spec)?;
            add += b.len();
            if self.bytes + add > MAX_BYTES {
                return Err("catalog byte limit exceeded".into());
            }
            staged.push((
                key,
                OwnedSpec {
                    owner: owner.clone(),
                    spec: spec.clone(),
                },
            ));
        }
        if self.bytes + add > MAX_BYTES {
            return Err("catalog byte limit exceeded".into());
        }
        for (k, v) in staged {
            self.entries.insert(k, v);
        }
        self.bytes += add;
        Ok(())
    }

    pub fn remove_owner(&mut self, owner: &ExtensionId) -> usize {
        let os = owner.to_string();
        let keys: Vec<(Kind, ContentId)> = self
            .entries
            .iter()
            .filter(|(_, v)| v.owner.to_string() == os)
            .map(|(k, _)| k.clone())
            .collect();
        let n = keys.len();
        for k in keys {
            if let Some(r) = self.entries.remove(&k) {
                if let Ok(b) = canonical_bytes(&r.spec) {
                    self.bytes = self.bytes.saturating_sub(b.len());
                }
            }
        }
        n
    }

    pub fn freeze(self) -> Result<Catalog, String> {
        build_catalog(self.entries)
    }
}

fn parse_state_key(key: &str, s: &StateSchema) -> Result<BTreeMap<String, String>, String> {
    let props = s.properties();
    if props.is_empty() {
        if key.is_empty() {
            return Ok(BTreeMap::new());
        }
        return Err("singleton block expects empty key".into());
    }
    if key.is_empty() {
        return Err("empty state key".into());
    }
    let mut out = BTreeMap::new();
    let mut names = Vec::new();
    for part in key.split(',') {
        let (k, v) = part.split_once('=').ok_or("malformed state key")?;
        if k.is_empty() || v.is_empty() {
            return Err("malformed state key".into());
        }
        if out.insert(k.to_string(), v.to_string()).is_some() {
            return Err("duplicated property in key".into());
        }
        names.push(k.to_string());
    }
    if out.len() != props.len() {
        return Err("missing/extra properties in key".into());
    }
    let mut sorted = names.clone();
    sorted.sort();
    if names != sorted {
        return Err("non-canonical key order".into());
    }
    s.validate_values(&out).map_err(|e| e.to_string())?;
    Ok(out)
}

fn check_value(
    ft: &FieldType,
    v: &serde_json::Value,
    m: &BTreeMap<(Kind, ContentId), OwnedSpec>,
    strict: bool,
) -> Result<(), String> {
    match ft {
        FieldType::Bool => v.is_boolean().then_some(()).ok_or("want bool".into()),
        FieldType::Integer { min, max } => {
            let n = v.as_i64().ok_or("want integer")?;
            (n >= *min && n <= *max)
                .then_some(())
                .ok_or("integer out of range".into())
        }
        FieldType::Text { max_bytes } => {
            let s: &str = v.as_str().ok_or("want text")?;
            (s.len() <= *max_bytes as usize)
                .then_some(())
                .ok_or("text too long".into())
        }
        FieldType::Content { kind } => {
            let s: &str = v.as_str().ok_or("want content ref")?;
            let cid = s
                .parse::<ContentId>()
                .map_err(|_| "bad content id".to_string())?;
            if strict {
                lookup(m, *kind, &cid)?;
            }
            Ok(())
        }
    }
}

fn check_components(
    m: &BTreeMap<(Kind, ContentId), OwnedSpec>,
    c: &Components,
) -> Result<(), String> {
    if c.len() > crate::spec::MAX_COMPONENTS_PER_SPEC {
        return Err("component count exceeded".into());
    }
    for (cid, vals) in c {
        let o = lookup(m, Kind::Component, cid)?;
        let decl = match &o.spec {
            ContentSpec::Component { fields, .. } => fields,
            _ => return Err("component kind mismatch".into()),
        };
        if vals.len() != decl.len() || !vals.keys().all(|k| decl.contains_key(k)) {
            return Err(format!("component field mismatch: {cid}"));
        }
        for (f, t) in decl {
            let v = vals.get(f).ok_or("missing component field")?;
            check_value(t, v, m, true)?;
        }
    }
    Ok(())
}

fn item_max(m: &BTreeMap<(Kind, ContentId), OwnedSpec>, id: &ContentId) -> Result<u16, String> {
    match &lookup(m, Kind::Item, id)?.spec {
        ContentSpec::Item { max_stack, .. } => Ok(*max_stack),
        _ => Err("item kind mismatch".into()),
    }
}

fn check_stack(
    m: &BTreeMap<(Kind, ContentId), OwnedSpec>,
    s: &InventoryStack,
) -> Result<(), String> {
    s.validate()?;
    let max = item_max(m, &s.item)?;
    if s.count < 1 || s.count > max {
        return Err(format!("stack count vs max_stack: {}", s.item));
    }
    Ok(())
}

fn resolve_tag(
    m: &BTreeMap<(Kind, ContentId), OwnedSpec>,
    tag: &ContentId,
    cache: &mut BTreeMap<ContentId, BTreeSet<ContentId>>,
    visiting: &mut BTreeSet<ContentId>,
    depth: usize,
) -> Result<BTreeSet<ContentId>, String> {
    if depth > MAX_TAG_DEPTH {
        return Err("tag nesting too deep".into());
    }
    if let Some(c) = cache.get(tag) {
        return Ok(c.clone());
    }
    if !visiting.insert(tag.clone()) {
        return Err(format!("tag cycle: {tag}"));
    }
    let o = lookup(m, Kind::Tag, tag)?;
    let (target, members, includes) = match &o.spec {
        ContentSpec::Tag {
            target,
            members,
            includes,
            ..
        } => (*target, members, includes),
        _ => return Err("tag kind mismatch".into()),
    };
    let mut out = BTreeSet::new();
    for x in members {
        lookup(m, target, x)?;
        out.insert(x.clone());
    }
    for inc in includes {
        let c = lookup(m, Kind::Tag, inc)?;
        let ct = match &c.spec {
            ContentSpec::Tag { target, .. } => *target,
            _ => return Err("tag kind mismatch".into()),
        };
        if ct != target {
            return Err("tag include target mismatch".into());
        }
        out.extend(resolve_tag(m, inc, cache, visiting, depth + 1)?);
    }
    visiting.remove(tag);
    if out.len() > MAX_TAG_RESOLVED {
        return Err("tag resolved member limit exceeded".into());
    }
    if cache.values().map(BTreeSet::len).sum::<usize>() + out.len() > 65536
        || cache
            .values()
            .flatten()
            .map(|id| id.as_str().len())
            .sum::<usize>()
            + out.iter().map(|id| id.as_str().len()).sum::<usize>()
            > MAX_BYTES
    {
        return Err("aggregate resolved tag limit exceeded".into());
    }
    cache.insert(tag.clone(), out.clone());
    Ok(out)
}

fn check_predicate_detail(
    m: &BTreeMap<(Kind, ContentId), OwnedSpec>,
    tag_of: &ContentId,
) -> Result<(), String> {
    let o = lookup(m, Kind::Condition, tag_of)?;
    let p = match &o.spec {
        ContentSpec::Condition { predicate, .. } => predicate,
        _ => return Err("condition kind mismatch".into()),
    };
    match p {
        crate::spec::Predicate::Always => Ok(()),
        crate::spec::Predicate::ComponentEquals {
            component,
            field,
            value,
        } => {
            let c = lookup(m, Kind::Component, component)?;
            let decl = match &c.spec {
                ContentSpec::Component { fields, .. } => fields,
                _ => return Err("component kind mismatch".into()),
            };
            let t = decl.get(field).ok_or("unknown component field")?;
            check_value(t, value, m, true)
        }
        crate::spec::Predicate::InTag { tag, content } => {
            let t = lookup(m, Kind::Tag, tag)?;
            let target = match &t.spec {
                ContentSpec::Tag { target, .. } => *target,
                _ => return Err("tag kind mismatch".into()),
            };
            lookup(m, target, content)?;
            Ok(())
        }
        crate::spec::Predicate::All { .. }
        | crate::spec::Predicate::Any { .. }
        | crate::spec::Predicate::Not { .. } => Ok(()),
    }
}

fn cond_children(
    m: &BTreeMap<(Kind, ContentId), OwnedSpec>,
    id: &ContentId,
) -> Result<Vec<ContentId>, String> {
    let o = lookup(m, Kind::Condition, id)?;
    Ok(match &o.spec {
        ContentSpec::Condition { predicate, .. } => match predicate {
            crate::spec::Predicate::All { conditions }
            | crate::spec::Predicate::Any { conditions } => conditions.clone(),
            crate::spec::Predicate::Not { condition } => vec![condition.clone()],
            _ => Vec::new(),
        },
        _ => return Err("condition kind mismatch".into()),
    })
}

fn graph_height(
    m: &BTreeMap<(Kind, ContentId), OwnedSpec>,
    kind: Kind,
    id: &ContentId,
    visiting: &mut BTreeSet<ContentId>,
    memo: &mut BTreeMap<ContentId, usize>,
) -> Result<usize, String> {
    let limit = if kind == Kind::Tag {
        MAX_TAG_DEPTH
    } else {
        MAX_COND_DEPTH
    };
    if visiting.len() > limit {
        return Err("reference graph depth exceeded".into());
    }
    if let Some(&height) = memo.get(id) {
        if visiting.len() + height > limit {
            return Err("reference graph depth exceeded".into());
        }
        return Ok(height);
    }
    if !visiting.insert(id.clone()) {
        return Err(format!("reference cycle: {id}"));
    }
    let children = if kind == Kind::Tag {
        match &lookup(m, kind, id)?.spec {
            ContentSpec::Tag { includes, .. } => includes.clone(),
            _ => return Err("tag kind mismatch".into()),
        }
    } else {
        cond_children(m, id)?
    };
    let mut height = 0;
    for child in children {
        height = height.max(1 + graph_height(m, kind, &child, visiting, memo)?);
    }
    visiting.remove(id);
    if visiting.len() + height > limit {
        return Err("reference graph depth exceeded".into());
    }
    memo.insert(id.clone(), height);
    Ok(height)
}

fn compute_fingerprint(m: &BTreeMap<(Kind, ContentId), OwnedSpec>) -> Result<String, String> {
    let mut h = sha2::Sha256::new();
    h.update(b"ELTHARN-M06-CATALOG-V1\0");
    for kind in KINDS {
        let kid = format!("eltharn:{}", kind.as_str())
            .parse::<ContentId>()
            .map_err(|e| e.to_string())?;
        let mut b = RegistryBuilder::new(kid, 1).map_err(|e| e.to_string())?;
        for ((k, id), o) in m.iter().filter(|((k, _), _)| *k == kind) {
            let raw = serde_json::to_vec(&o.spec).map_err(|e| e.to_string())?;
            let def = Definition::parse(&raw).map_err(|e| e.to_string())?;
            b.register_batch(o.owner.clone(), vec![(id.clone(), def)])
                .map_err(|e| e.to_string())?;
            let _ = k;
        }
        let frozen = b.freeze();
        let hex = frozen.fingerprint().to_string();
        h.update((kind.as_str().len() as u64).to_be_bytes());
        h.update(kind.as_str().as_bytes());
        h.update((hex.len() as u64).to_be_bytes());
        h.update(hex.as_bytes());
    }
    let mut s = String::with_capacity(64);
    for x in h.finalize() {
        let _ = write!(s, "{x:02x}");
    }
    Ok(s)
}

fn build_catalog(entries: BTreeMap<(Kind, ContentId), OwnedSpec>) -> Result<Catalog, String> {
    if entries.len() > MAX_ENTRIES {
        return Err("catalog entry limit exceeded".into());
    }
    let mut schemas: BTreeMap<ContentId, StateSchema> = BTreeMap::new();
    let mut total = 0usize;
    for ((k, id), o) in &entries {
        if *k != Kind::Block {
            continue;
        }
        if let ContentSpec::Block {
            states,
            state_models,
            ..
        } = &o.spec
        {
            let s = states.schema().map_err(|e| e.to_string())?;
            total += s.state_count();
            if total > MAX_TOTAL_STATES {
                return Err("total block states exceeded".into());
            }
            for key in state_models.keys() {
                parse_state_key(key, &s)?;
            }
            schemas.insert(id.clone(), s);
        }
    }
    for o in entries.values() {
        if let Some(c) = o.spec.component_defaults() {
            check_components(&entries, c)?;
        }
    }
    for ((_, _), o) in &entries {
        match &o.spec {
            ContentSpec::Recipe { inputs, output, .. } => {
                for s in inputs {
                    check_stack(&entries, s)?;
                }
                check_stack(&entries, output)?;
            }
            ContentSpec::Loot { entries: le, .. } => {
                for e in le {
                    check_stack(&entries, &e.stack)?;
                }
            }
            _ => {}
        }
    }
    for o in entries.values() {
        for (rk, rid) in o.spec.references() {
            lookup(&entries, rk, &rid)?;
        }
    }
    let mut cache: BTreeMap<ContentId, BTreeSet<ContentId>> = BTreeMap::new();
    let tags: Vec<ContentId> = entries
        .iter()
        .filter(|((k, _), _)| *k == Kind::Tag)
        .map(|((_, id), _)| id.clone())
        .collect();
    let mut tag_heights = BTreeMap::new();
    for t in &tags {
        graph_height(
            &entries,
            Kind::Tag,
            t,
            &mut BTreeSet::new(),
            &mut tag_heights,
        )?;
        resolve_tag(&entries, t, &mut cache, &mut BTreeSet::new(), 0)?;
    }
    let conds: Vec<ContentId> = entries
        .iter()
        .filter(|((k, _), _)| *k == Kind::Condition)
        .map(|((_, id), _)| id.clone())
        .collect();
    for c in &conds {
        check_predicate_detail(&entries, c)?;
    }
    let mut heights = BTreeMap::new();
    for c in &conds {
        graph_height(
            &entries,
            Kind::Condition,
            c,
            &mut BTreeSet::new(),
            &mut heights,
        )?;
    }
    let fp = compute_fingerprint(&entries)?;
    Ok(Catalog {
        entries,
        fingerprint: fp,
        resolved_tags: cache,
        schemas,
    })
}

impl Catalog {
    pub fn from_entries(list: Vec<OwnedSpec>) -> Result<Self, String> {
        if list.len() > MAX_ENTRIES {
            return Err("catalog entry limit exceeded".into());
        }
        let mut grouped: BTreeMap<ExtensionId, Vec<ContentSpec>> = BTreeMap::new();
        for o in list {
            grouped.entry(o.owner).or_default().push(o.spec);
        }
        let mut b = CatalogBuilder::default();
        let owners: Vec<ExtensionId> = {
            let mut v: Vec<ExtensionId> = grouped.keys().cloned().collect();
            v.sort_by(|a, c| a.to_string().cmp(&c.to_string()));
            v
        };
        for o in owners {
            let batch = grouped.remove(&o).unwrap_or_default();
            b.register_batch(o, batch)?;
        }
        b.freeze()
    }
    pub fn get(&self, kind: Kind, id: &ContentId) -> Result<&ContentSpec, String> {
        Ok(&lookup(&self.entries, kind, id)?.spec)
    }
    pub fn owner(&self, kind: Kind, id: &ContentId) -> Result<&ExtensionId, String> {
        Ok(&lookup(&self.entries, kind, id)?.owner)
    }
    pub fn entries(&self) -> impl Iterator<Item = &OwnedSpec> {
        self.entries.values()
    }
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn contains(&self, kind: Kind, id: &ContentId) -> bool {
        self.entries.contains_key(&(kind, id.clone()))
    }
    pub fn tag_members(&self, tag: &ContentId) -> Result<&BTreeSet<ContentId>, String> {
        self.resolved_tags
            .get(tag)
            .ok_or_else(|| format!("unknown tag: {tag}"))
    }
    pub fn block_state_key(
        &self,
        id: &ContentId,
        values: &BTreeMap<String, String>,
    ) -> Result<String, String> {
        let s = self.schemas.get(id).ok_or("unknown block")?;
        s.validate_values(values).map_err(|e| e.to_string())?;
        if s.properties().is_empty() {
            return Ok(String::new());
        }
        Ok(values
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join(","))
    }
    pub fn validate_components(&self, c: &Components) -> Result<(), String> {
        check_components(&self.entries, c)
    }
    pub fn validate_stack(&self, s: &InventoryStack) -> Result<(), String> {
        check_stack(&self.entries, s)
    }
    pub fn export(&self) -> Vec<OwnedSpec> {
        self.entries.values().cloned().collect()
    }
}
