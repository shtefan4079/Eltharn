use core::{fmt, ops::Range};
pub const ABI_GENERATION: u32 = 1;
pub const HOST_MODULE: &str = "rustore_host_v1";
pub const HOST_CALL_FN: &str = "call";
pub const MAX_MESSAGE_BYTES: u32 = 65_536;
pub const MAX_BATCH_RECORDS: u32 = 4_096;
pub const MEMORY_MAX_LEN: u64 = 0x1_0000_0000;
pub const DESCRIPTOR_LEN: usize = 16;
pub const BATCH_HEADER_LEN: usize = 16;
pub const BATCH_VERSION: u32 = 1;
pub const EXPORT_MEMORY: &str = "memory";
pub const EXPORT_INIT: &str = "init";
pub const EXPORT_TICK: &str = "tick";
pub const EXPORT_SHUTDOWN: &str = "shutdown";
pub const EXPORT_WORLD_ENTER: &str = "on_world_enter";
pub const EXPORT_WORLD_EXIT: &str = "on_world_exit";
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    InvalidOpcode = -1,
    InvalidRequest = -2,
    OutOfBounds = -3,
    BufferTooSmall = -4,
    CapabilityDenied = -5,
    WrongPhase = -6,
    StaleHandle = -7,
    LimitExceeded = -8,
    HostFailure = -9,
    Unavailable = -10,
}
impl ErrorCode {
    pub fn code(self) -> i32 {
        self as i32
    }
    pub fn from_code(code: i32) -> Option<Self> {
        match code {
            -1 => Some(Self::InvalidOpcode),
            -2 => Some(Self::InvalidRequest),
            -3 => Some(Self::OutOfBounds),
            -4 => Some(Self::BufferTooSmall),
            -5 => Some(Self::CapabilityDenied),
            -6 => Some(Self::WrongPhase),
            -7 => Some(Self::StaleHandle),
            -8 => Some(Self::LimitExceeded),
            -9 => Some(Self::HostFailure),
            -10 => Some(Self::Unavailable),
            _ => None,
        }
    }
}
impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::InvalidOpcode => "invalid opcode",
            Self::InvalidRequest => "invalid request",
            Self::OutOfBounds => "out of bounds",
            Self::BufferTooSmall => "buffer too small",
            Self::CapabilityDenied => "capability denied",
            Self::WrongPhase => "wrong phase",
            Self::StaleHandle => "stale handle",
            Self::LimitExceeded => "limit exceeded",
            Self::HostFailure => "host failure",
            Self::Unavailable => "unavailable",
        };
        write!(f, "{s} ({})", self.code())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GuestRange {
    offset: u32,
    length: u32,
}
impl GuestRange {
    pub fn new(offset: u32, length: u32) -> Self {
        Self { offset, length }
    }
    pub fn from_wasm_params(offset: i32, length: i32) -> Self {
        Self {
            offset: offset as u32,
            length: length as u32,
        }
    }
    pub fn offset(self) -> u32 {
        self.offset
    }
    pub fn length(self) -> u32 {
        self.length
    }
    pub fn end_u64(self) -> u64 {
        self.offset as u64 + self.length as u64
    }
    pub fn check(self, memory_len: u64) -> Result<(), ErrorCode> {
        if memory_len > MEMORY_MAX_LEN {
            return Err(ErrorCode::HostFailure);
        }
        if self.length > MAX_MESSAGE_BYTES {
            return Err(ErrorCode::LimitExceeded);
        }
        if self.end_u64() > memory_len {
            return Err(ErrorCode::OutOfBounds);
        }
        Ok(())
    }
    pub fn resolve(self, memory_len: u64) -> Result<Range<usize>, ErrorCode> {
        self.check(memory_len)?;
        let start = usize::try_from(self.offset as u64).map_err(|_| ErrorCode::OutOfBounds)?;
        let end = usize::try_from(self.end_u64()).map_err(|_| ErrorCode::OutOfBounds)?;
        Ok(start..end)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HostCallLayout {
    request: GuestRange,
    response: GuestRange,
}
impl HostCallLayout {
    pub fn validate(
        request_ptr: i32,
        request_len: i32,
        response_ptr: i32,
        response_capacity: i32,
        memory_len: u64,
    ) -> Result<Self, ErrorCode> {
        if memory_len > MEMORY_MAX_LEN {
            return Err(ErrorCode::HostFailure);
        }
        let request = GuestRange::from_wasm_params(request_ptr, request_len);
        let response = GuestRange::from_wasm_params(response_ptr, response_capacity);
        request.check(memory_len)?;
        response.check(memory_len)?;
        Ok(Self { request, response })
    }
    pub fn request(self) -> GuestRange {
        self.request
    }
    pub fn response(self) -> GuestRange {
        self.response
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Descriptor {
    owner: u32,
    slot: u32,
    generation: u32,
    kind: u32,
}
impl Descriptor {
    pub fn new(owner: u32, slot: u32, generation: u32, kind: u32) -> Result<Self, ErrorCode> {
        if owner == 0 || generation == 0 || kind == 0 {
            return Err(ErrorCode::StaleHandle);
        }
        Ok(Self {
            owner,
            slot,
            generation,
            kind,
        })
    }
    pub fn owner(self) -> u32 {
        self.owner
    }
    pub fn slot(self) -> u32 {
        self.slot
    }
    pub fn generation(self) -> u32 {
        self.generation
    }
    pub fn kind(self) -> u32 {
        self.kind
    }
    pub fn encode(self) -> [u8; DESCRIPTOR_LEN] {
        let mut out = [0u8; DESCRIPTOR_LEN];
        for (chunk, word) in
            out.chunks_exact_mut(4)
                .zip([self.owner, self.slot, self.generation, self.kind])
        {
            chunk.copy_from_slice(&word.to_le_bytes());
        }
        out
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, ErrorCode> {
        if bytes.len() != DESCRIPTOR_LEN {
            return Err(ErrorCode::InvalidRequest);
        }
        let mut words = [0u32; 4];
        for (word, chunk) in words.iter_mut().zip(bytes.chunks_exact(4)) {
            let mut raw = [0u8; 4];
            raw.copy_from_slice(chunk);
            *word = u32::from_le_bytes(raw);
        }
        let [owner, slot, generation, kind] = words;
        Self::new(owner, slot, generation, kind)
    }
    pub fn check_scope(self, expected_owner: u32, expected_kind: u32) -> Result<(), ErrorCode> {
        if self.owner != expected_owner || self.kind != expected_kind {
            return Err(ErrorCode::StaleHandle);
        }
        Ok(())
    }
}
fn batch_total_len(count: u32, record_size: u32) -> Result<usize, ErrorCode> {
    if count > MAX_BATCH_RECORDS {
        return Err(ErrorCode::LimitExceeded);
    }
    if record_size == 0 {
        return Err(ErrorCode::InvalidRequest);
    }
    let body = count as u64 * record_size as u64;
    let total = body + BATCH_HEADER_LEN as u64;
    if total > MAX_MESSAGE_BYTES as u64 {
        return Err(ErrorCode::LimitExceeded);
    }
    usize::try_from(total).map_err(|_| ErrorCode::OutOfBounds)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatchView<'a> {
    count: u32,
    record_size: u32,
    payload: &'a [u8],
}
impl<'a> BatchView<'a> {
    pub fn decode(bytes: &'a [u8], expected_record_size: Option<u32>) -> Result<Self, ErrorCode> {
        if bytes.len() > MAX_MESSAGE_BYTES as usize {
            return Err(ErrorCode::LimitExceeded);
        }
        if bytes.len() < BATCH_HEADER_LEN {
            return Err(ErrorCode::InvalidRequest);
        }
        let (header, rest) = bytes.split_at(BATCH_HEADER_LEN);
        let mut words = [0u32; 4];
        for (word, chunk) in words.iter_mut().zip(header.chunks_exact(4)) {
            let mut raw = [0u8; 4];
            raw.copy_from_slice(chunk);
            *word = u32::from_le_bytes(raw);
        }
        let [version, count, record_size, reserved] = words;
        if version != BATCH_VERSION {
            return Err(ErrorCode::InvalidRequest);
        }
        if reserved != 0 {
            return Err(ErrorCode::InvalidRequest);
        }
        if let Some(expected) = expected_record_size {
            if record_size != expected {
                return Err(ErrorCode::InvalidRequest);
            }
        }
        let total = batch_total_len(count, record_size)?;
        if rest.len() != total - BATCH_HEADER_LEN {
            return Err(ErrorCode::InvalidRequest);
        }
        Ok(Self {
            count,
            record_size,
            payload: rest,
        })
    }
    pub fn count(self) -> u32 {
        self.count
    }
    pub fn record_size(self) -> u32 {
        self.record_size
    }
    pub fn payload(self) -> &'a [u8] {
        self.payload
    }
    pub fn record(self, index: u32) -> Option<&'a [u8]> {
        if index >= self.count {
            return None;
        }
        let width = self.record_size as usize;
        let start = index as usize * width;
        self.payload.get(start..start + width)
    }
}
pub fn batch_encode_into(
    count: u32,
    record_size: u32,
    payload: &[u8],
    dest: &mut [u8],
) -> Result<usize, ErrorCode> {
    let total = batch_total_len(count, record_size)?;
    if payload.len() != total - BATCH_HEADER_LEN {
        return Err(ErrorCode::InvalidRequest);
    }
    if dest.len() < total {
        return Err(ErrorCode::BufferTooSmall);
    }
    let mut header = [0u8; BATCH_HEADER_LEN];
    for (chunk, word) in header
        .chunks_exact_mut(4)
        .zip([BATCH_VERSION, count, record_size, 0u32])
    {
        chunk.copy_from_slice(&word.to_le_bytes());
    }
    let (head, tail) = dest.split_at_mut(BATCH_HEADER_LEN);
    head.copy_from_slice(&header);
    let slot = tail
        .get_mut(..payload.len())
        .ok_or(ErrorCode::HostFailure)?;
    slot.copy_from_slice(payload);
    Ok(total)
}
