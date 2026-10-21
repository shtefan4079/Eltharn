use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub version: u32,
    pub max_actors: usize,
    pub memory_total: u64,
    pub init_fuel: u64,
    pub callback_fuel: u64,
    pub stack_bytes: usize,
}

impl Default for Policy {
    fn default() -> Self {
        Self {
            version: 2,
            max_actors: 16,
            memory_total: 512 * 1024 * 1024,
            init_fuel: 10_000_000,
            callback_fuel: 2_000_000,
            stack_bytes: 256 * 1024,
        }
    }
}

impl Policy {
    pub fn validate(&self) -> Result<(), String> {
        if self != &Self::default() {
            return Err("Unsupported mod host policy; prepare again with policy v2".into());
        }
        Ok(())
    }
}

pub const MAX_CALLS: usize = 128;
pub const MAX_OPERATIONS: usize = 64;
pub const MAX_PAYLOAD_BYTES: usize = 65536;
pub const MAX_REGISTRATIONS: usize = 64;
pub const MAX_JOBS: usize = 2;

pub fn memory_reservation(bytes: &[u8]) -> Result<u64, String> {
    crate::admission::validate(bytes).map_err(|e| e.to_string())?;
    for payload in wasmparser::Parser::new(0).parse_all(bytes) {
        if let wasmparser::Payload::MemorySection(section) = payload.map_err(|e| e.to_string())? {
            let memory = section
                .into_iter()
                .next()
                .ok_or("Missing memory")?
                .map_err(|e| e.to_string())?;
            return memory
                .maximum
                .ok_or("Missing memory maximum".into())
                .map(|pages| pages * 65536);
        }
    }
    Err("Missing memory".into())
}
