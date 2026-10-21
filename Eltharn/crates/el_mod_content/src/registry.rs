use std::collections::{BTreeMap, HashMap, HashSet};

use el_extension_api::ExtensionId;
use sha2::{Digest, Sha256};

use crate::{ContentId, Definition};

pub const MAX_OWNER_ENTRIES: usize = 512;
pub const MAX_ENTRIES: usize = 4096;
pub const MAX_TOTAL_BYTES: usize = 4194304;

pub struct Entry {
    id: ContentId,
    owner: ExtensionId,
    definition: Definition,
}

impl Entry {
    pub fn id(&self) -> &ContentId {
        &self.id
    }
    pub fn owner(&self) -> &ExtensionId {
        &self.owner
    }
    pub fn definition(&self) -> &Definition {
        &self.definition
    }
}

pub struct RegistryBuilder {
    kind: ContentId,
    schema_version: u32,
    entries: BTreeMap<ContentId, Entry>,
    total_bytes: usize,
}

impl RegistryBuilder {
    pub fn new(kind: ContentId, schema_version: u32) -> Result<Self, String> {
        if schema_version == 0 {
            return Err("schema_version must be nonzero".to_string());
        }
        Ok(Self {
            kind,
            schema_version,
            entries: BTreeMap::new(),
            total_bytes: 0,
        })
    }
    pub fn kind(&self) -> &ContentId {
        &self.kind
    }
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn contains(&self, id: &ContentId) -> bool {
        self.entries.contains_key(id)
    }
    pub fn total_definition_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn register_batch(
        &mut self,
        owner: ExtensionId,
        batch: Vec<(ContentId, Definition)>,
    ) -> Result<(), String> {
        if owner.is_reserved_namespace() {
            return Err("reserved owner namespace".to_string());
        }
        let owner_existing = self.entries.values().filter(|e| e.owner == owner).count();
        if owner_existing
            .checked_add(batch.len())
            .is_none_or(|n| n > MAX_OWNER_ENTRIES)
        {
            return Err("per-owner entry limit exceeded".to_string());
        }
        if self
            .entries
            .len()
            .checked_add(batch.len())
            .is_none_or(|n| n > MAX_ENTRIES)
        {
            return Err("global entry limit exceeded".to_string());
        }
        let mut seen = HashSet::with_capacity(batch.len());
        for (id, _) in &batch {
            if id.is_reserved_namespace() {
                return Err(format!("reserved id: {id}"));
            }
            if id.namespace() != owner.namespace() {
                return Err(format!("owner namespace mismatch: {id}"));
            }
            if self.entries.contains_key(id) {
                return Err(format!("duplicate id: {id}"));
            }
            if !seen.insert(id) {
                return Err(format!("duplicate id in batch: {id}"));
            }
        }
        let mut staged = self.total_bytes;
        for (_, def) in &batch {
            staged = staged
                .checked_add(def.canonical_bytes().len())
                .ok_or_else(|| "total byte limit exceeded".to_string())?;
            if staged > MAX_TOTAL_BYTES {
                return Err("total byte limit exceeded".to_string());
            }
        }
        for (id, def) in batch {
            let key = id.clone();
            self.entries.insert(
                key,
                Entry {
                    id,
                    owner: owner.clone(),
                    definition: def,
                },
            );
        }
        self.total_bytes = staged;
        Ok(())
    }

    pub fn remove_owner(&mut self, owner: &ExtensionId) -> usize {
        let doomed: Vec<ContentId> = self
            .entries
            .iter()
            .filter(|(_, e)| &e.owner == owner)
            .map(|(k, _)| k.clone())
            .collect();
        let mut freed = 0;
        for k in &doomed {
            if let Some(e) = self.entries.remove(k) {
                freed += e.definition.canonical_bytes().len();
            }
        }
        self.total_bytes -= freed;
        doomed.len()
    }

    pub fn freeze(self) -> FrozenRegistry {
        let entries: Vec<Entry> = self.entries.into_values().collect();
        let total = self.total_bytes;
        let kind = self.kind;
        let schema = self.schema_version;
        let mut h = Sha256::new();
        h.update(b"ELTHARN-M06-02-A-REGISTRY-V1\0");
        h.update((kind.as_str().len() as u64).to_be_bytes());
        h.update(kind.as_str().as_bytes());
        h.update(schema.to_be_bytes());
        h.update((entries.len() as u64).to_be_bytes());
        for e in &entries {
            h.update((e.id.as_str().len() as u64).to_be_bytes());
            h.update(e.id.as_str().as_bytes());
            h.update((e.owner.as_str().len() as u64).to_be_bytes());
            h.update(e.owner.as_str().as_bytes());
            let cb = e.definition.canonical_bytes();
            h.update((cb.len() as u64).to_be_bytes());
            h.update(cb);
        }
        let digest: [u8; 32] = h.finalize().into();
        let mut lookup = HashMap::with_capacity(entries.len() * 2 + 1);
        for (i, e) in entries.iter().enumerate() {
            lookup.insert(e.id.clone(), i as u32);
        }
        FrozenRegistry {
            kind,
            schema_version: schema,
            entries,
            lookup,
            digest,
            total_bytes: total,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ContentHandle {
    digest: [u8; 32],
    index: u32,
}

pub struct FrozenRegistry {
    kind: ContentId,
    schema_version: u32,
    entries: Vec<Entry>,
    lookup: HashMap<ContentId, u32>,
    digest: [u8; 32],
    total_bytes: usize,
}

fn hex32(d: &[u8; 32]) -> String {
    const H: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(64);
    for &b in d {
        s.push(H[(b >> 4) as usize] as char);
        s.push(H[(b & 15) as usize] as char);
    }
    s
}

impl FrozenRegistry {
    pub fn fingerprint(&self) -> String {
        hex32(&self.digest)
    }
    pub fn kind(&self) -> &ContentId {
        &self.kind
    }
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn total_definition_bytes(&self) -> usize {
        self.total_bytes
    }
    pub fn iter(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }
    pub fn lookup(&self, id: &ContentId) -> Result<(&Entry, ContentHandle), String> {
        let &idx = self
            .lookup
            .get(id)
            .ok_or_else(|| format!("unknown id: {id}"))?;
        let e = self
            .entries
            .get(idx as usize)
            .ok_or_else(|| "corrupt index".to_string())?;
        Ok((
            e,
            ContentHandle {
                digest: self.digest,
                index: idx,
            },
        ))
    }
    pub fn get(&self, handle: ContentHandle) -> Result<&Entry, String> {
        if handle.digest != self.digest {
            return Err("stale handle digest".to_string());
        }
        self.entries
            .get(handle.index as usize)
            .ok_or_else(|| "bad handle index".to_string())
    }
}
