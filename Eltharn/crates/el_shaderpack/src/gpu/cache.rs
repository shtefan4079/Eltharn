use super::Result;
use super::pipelines::Pipelines;
use crate::model::{ValidatedPack, Variant};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
const MAX_CAPACITY: usize = 64;
fn sha256(bytes: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().into()
}
fn hex(bytes: &[u8; 32]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 15) as usize] as char);
    }
    out
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramFingerprint {
    pub program: String,
    pub source_digest: [u8; 32],
    pub vertex_entry: String,
    pub fragment_entry: String,
    pub has_params: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PipelineCacheKey {
    pub pack_id: String,
    pub content_id: String,
    pub profile: String,
    pub variant: String,
    pub programs: Vec<ProgramFingerprint>,
    pub params_digest: Option<[u8; 32]>,
    pub scene_format: String,
}
impl PipelineCacheKey {
    pub fn for_pack(
        pack: &ValidatedPack,
        variant: &Variant,
        scene_format: wgpu::TextureFormat,
    ) -> Result<Self> {
        let mut seen = BTreeSet::new();
        let mut programs = Vec::new();
        for pass in &variant.passes {
            if !seen.insert(pass.program.clone()) {
                continue;
            }
            let program = pack.programs.get(&pass.program).ok_or_else(|| {
                format!(
                    "pipeline key: pack '{}' pass '{}' references unknown program '{}'",
                    pack.id, pass.id, pass.program
                )
            })?;
            programs.push(ProgramFingerprint {
                program: pass.program.clone(),
                source_digest: sha256(program.source.as_bytes()),
                vertex_entry: program.vertex_entry.clone(),
                fragment_entry: program.fragment_entry.clone(),
                has_params: program.has_params,
            });
        }
        programs.sort_by(|a, b| a.program.cmp(&b.program));
        Ok(Self {
            pack_id: pack.id.clone(),
            content_id: pack.content_id.clone(),
            profile: pack.profile.name().to_string(),
            variant: variant.id.clone(),
            programs,
            params_digest: variant.params.as_ref().map(|bytes| sha256(bytes)),
            scene_format: format!("{scene_format:?}"),
        })
    }
    pub fn digest(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"rustore.pipeline_key.v1\0");
        for part in [
            &self.pack_id,
            &self.content_id,
            &self.profile,
            &self.variant,
        ] {
            hasher.update(part.as_bytes());
            hasher.update([0]);
        }
        for program in &self.programs {
            hasher.update(program.program.as_bytes());
            hasher.update([0]);
            hasher.update(program.source_digest);
            hasher.update(program.vertex_entry.as_bytes());
            hasher.update([0]);
            hasher.update(program.fragment_entry.as_bytes());
            hasher.update([0]);
            hasher.update([u8::from(program.has_params)]);
        }
        hasher.update([0]);
        match &self.params_digest {
            Some(digest) => {
                hasher.update([1]);
                hasher.update(digest);
            }
            None => hasher.update([0]),
        }
        hasher.update(self.scene_format.as_bytes());
        hasher.finalize().into()
    }
    pub fn digest_hex(&self) -> String {
        hex(&self.digest())
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub len: usize,
    pub capacity: usize,
}
#[derive(Debug)]
pub struct PipelineCache {
    capacity: usize,
    entries: BTreeMap<PipelineCacheKey, Pipelines>,
    order: VecDeque<PipelineCacheKey>,
    hits: u64,
    misses: u64,
    evictions: u64,
}
impl PipelineCache {
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.clamp(1, MAX_CAPACITY),
            entries: BTreeMap::new(),
            order: VecDeque::new(),
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn contains(&self, key: &PipelineCacheKey) -> bool {
        self.entries.contains_key(key)
    }
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            len: self.entries.len(),
            capacity: self.capacity,
        }
    }
    pub(crate) fn take(&mut self, key: &PipelineCacheKey) -> Option<Pipelines> {
        match self.entries.remove(key) {
            Some(pipes) => {
                self.order.retain(|queued| queued != key);
                self.hits += 1;
                Some(pipes)
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }
    pub(crate) fn store(&mut self, key: PipelineCacheKey, pipes: Pipelines) {
        if self.entries.insert(key.clone(), pipes).is_none() {
            while self.entries.len() > self.capacity {
                if let Some(oldest) = self.order.pop_front() {
                    if self.entries.remove(&oldest).is_some() {
                        self.evictions += 1;
                    }
                } else {
                    break;
                }
            }
        } else {
            self.order.retain(|queued| queued != &key);
        }
        self.order.push_back(key);
    }
    pub fn invalidate(&mut self, key: &PipelineCacheKey) -> bool {
        if self.entries.remove(key).is_some() {
            self.order.retain(|queued| queued != key);
            true
        } else {
            false
        }
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }
}
