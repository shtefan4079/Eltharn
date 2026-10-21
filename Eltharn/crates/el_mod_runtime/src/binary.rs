use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static INSTANCE_COUNTER: AtomicU64 = AtomicU64::new(1);

pub const CAPABILITY: &str = el_extension_api::services::BINARY_BUFFERS_CAPABILITY;
pub const OPCODE: i32 = el_extension_api::services::BINARY_BUFFERS_OPCODE;
pub const MAX_BUFFER: usize = 65536;
pub const MAX_BYTES: usize = 1024 * 1024;
pub const MAX_BUFFERS: usize = 64;
pub const MAX_PAGE: usize = 64;
pub const MAX_WORK: usize = 2 * 1024 * 1024;
pub const MAX_WIRE: usize = MAX_BYTES + 82 + MAX_BUFFERS * 20;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Endian {
    Little,
    Big,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scalar {
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
}
impl Scalar {
    pub fn width(self) -> usize {
        match self {
            Self::U8 => 1,
            Self::I16 | Self::U16 => 2,
            Self::I32 | Self::U32 | Self::F32 => 4,
            _ => 8,
        }
    }
    pub fn encode(self, value: &serde_json::Value, endian: Endian) -> Result<Vec<u8>, String> {
        macro_rules! signed {
            ($t:ty) => {{
                let n = <$t>::try_from(value.as_i64().ok_or("Expected integer scalar")?)
                    .map_err(|_| "Integer scalar range")?;
                match endian {
                    Endian::Big => n.to_be_bytes().to_vec(),
                    Endian::Little => n.to_le_bytes().to_vec(),
                }
            }};
        }
        macro_rules! unsigned {
            ($t:ty) => {{
                let n = <$t>::try_from(value.as_u64().ok_or("Expected unsigned scalar")?)
                    .map_err(|_| "Integer scalar range")?;
                match endian {
                    Endian::Big => n.to_be_bytes().to_vec(),
                    Endian::Little => n.to_le_bytes().to_vec(),
                }
            }};
        }
        macro_rules! wide {
            ($t:ty) => {{
                let n: $t = value
                    .as_str()
                    .ok_or("64-bit scalar must be a decimal string")?
                    .parse()
                    .map_err(|_| "64-bit scalar range")?;
                match endian {
                    Endian::Big => n.to_be_bytes().to_vec(),
                    Endian::Little => n.to_le_bytes().to_vec(),
                }
            }};
        }
        macro_rules! float {
            ($t:ty) => {{
                let n = value.as_f64().ok_or("Expected floating scalar")? as $t;
                if !n.is_finite() {
                    return Err("Nonfinite binary scalar".into());
                }
                match endian {
                    Endian::Big => n.to_be_bytes().to_vec(),
                    Endian::Little => n.to_le_bytes().to_vec(),
                }
            }};
        }
        Ok(match self {
            Self::U8 => unsigned!(u8),
            Self::I16 => signed!(i16),
            Self::U16 => unsigned!(u16),
            Self::I32 => signed!(i32),
            Self::U32 => unsigned!(u32),
            Self::I64 => wide!(i64),
            Self::U64 => wide!(u64),
            Self::F32 => float!(f32),
            Self::F64 => float!(f64),
        })
    }
    pub fn decode(self, bytes: &[u8], endian: Endian) -> Result<serde_json::Value, String> {
        if bytes.len() != self.width() {
            return Err("Binary scalar width".into());
        }
        macro_rules! number {
            ($t:ty) => {{
                match endian {
                    Endian::Big => <$t>::from_be_bytes(bytes.try_into().unwrap()),
                    Endian::Little => <$t>::from_le_bytes(bytes.try_into().unwrap()),
                }
            }};
        }
        Ok(match self {
            Self::U8 => serde_json::json!(bytes[0]),
            Self::I16 => serde_json::json!(number!(i16)),
            Self::U16 => serde_json::json!(number!(u16)),
            Self::I32 => serde_json::json!(number!(i32)),
            Self::U32 => serde_json::json!(number!(u32)),
            Self::I64 => serde_json::json!(number!(i64).to_string()),
            Self::U64 => serde_json::json!(number!(u64).to_string()),
            Self::F32 => {
                let n = number!(f32);
                if !n.is_finite() {
                    return Err("Nonfinite binary scalar".into());
                }
                serde_json::json!(n)
            }
            Self::F64 => {
                let n = number!(f64);
                if !n.is_finite() {
                    return Err("Nonfinite binary scalar".into());
                }
                serde_json::json!(n)
            }
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Mutation {
    Copy {
        source: String,
        destination: String,
        source_offset: usize,
        destination_offset: usize,
        count: usize,
    },
    Utf8 {
        text: String,
    },
    ScalarWrite {
        handle: String,
        offset: usize,
        scalar: Scalar,
        endian: Endian,
        value: serde_json::Value,
    },
    Create {
        size: usize,
    },
    Write {
        handle: String,
        offset: usize,
        bytes: Vec<u8>,
    },
    Slice {
        handle: String,
        offset: usize,
        count: usize,
    },
    Concat {
        handles: Vec<String>,
    },
    Release {
        handle: String,
    },
}
impl Mutation {
    pub fn validate(&self) -> Result<(), String> {
        let handle = |h: &str| !h.is_empty() && h.len() <= 96;
        let valid = match self {
            Self::Copy {
                source,
                destination,
                source_offset,
                destination_offset,
                count,
            } => {
                handle(source)
                    && handle(destination)
                    && *source_offset <= MAX_BUFFER
                    && *destination_offset <= MAX_BUFFER
                    && *count <= MAX_BUFFER
            }
            Self::Utf8 { text } => text.len() <= MAX_BUFFER,
            Self::ScalarWrite {
                handle: h,
                offset,
                scalar,
                endian,
                value,
            } => handle(h) && *offset <= MAX_BUFFER && scalar.encode(value, *endian).is_ok(),
            Self::Create { size } => *size <= MAX_BUFFER,
            Self::Write {
                handle: h,
                offset,
                bytes,
            } => handle(h) && *offset <= MAX_BUFFER && bytes.len() <= MAX_PAGE,
            Self::Slice {
                handle: h,
                offset,
                count,
            } => handle(h) && *offset <= MAX_BUFFER && *count <= MAX_BUFFER,
            Self::Concat { handles } => {
                !handles.is_empty() && handles.len() <= 16 && handles.iter().all(|h| handle(h))
            }
            Self::Release { handle: h } => handle(h),
        };
        valid
            .then_some(())
            .ok_or_else(|| "Binary request limit".into())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub version: u32,
    pub handle: String,
    pub size: usize,
    pub revision: String,
}
#[derive(Clone)]
struct Buffer {
    data: Arc<[u8]>,
    revision: u64,
}
#[derive(Clone)]
pub struct Store {
    namespace: String,
    serial: u64,
    bytes: usize,
    entries: BTreeMap<String, Buffer>,
}
impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinaryStore")
            .field("buffers", &self.entries.len())
            .field("bytes", &self.bytes)
            .finish_non_exhaustive()
    }
}
impl Store {
    pub fn new(owner: &str) -> Self {
        let mut digest = Sha256::new();
        digest.update(owner.as_bytes());
        digest.update(
            INSTANCE_COUNTER
                .fetch_add(1, Ordering::Relaxed)
                .to_le_bytes(),
        );
        let namespace = digest
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Self {
            namespace,
            serial: 0,
            bytes: 0,
            entries: BTreeMap::new(),
        }
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }
    pub fn to_wire(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.bytes + 82 + self.entries.len() * 20);
        out.extend_from_slice(b"ROB1");
        out.extend_from_slice(self.namespace.as_bytes());
        out.extend_from_slice(&self.serial.to_le_bytes());
        out.extend_from_slice(&(self.entries.len() as u16).to_le_bytes());
        for (handle, buffer) in &self.entries {
            let generation: u64 = handle.rsplit('/').next().unwrap().parse().unwrap();
            out.extend_from_slice(&generation.to_le_bytes());
            out.extend_from_slice(&buffer.revision.to_le_bytes());
            out.extend_from_slice(&(buffer.data.len() as u32).to_le_bytes());
            out.extend_from_slice(&buffer.data);
        }
        out
    }
    pub fn from_wire(wire: &[u8]) -> Result<Self, String> {
        fn take<'a>(input: &mut &'a [u8], count: usize) -> Result<&'a [u8], String> {
            let part = input
                .get(..count)
                .ok_or("Truncated binary resource channel")?;
            *input = &input[count..];
            Ok(part)
        }
        if wire.len() > MAX_WIRE {
            return Err("Binary resource channel limit".into());
        }
        let mut input = wire;
        if take(&mut input, 4)? != b"ROB1" {
            return Err("Binary resource channel version".into());
        }
        let namespace = std::str::from_utf8(take(&mut input, 64)?)
            .map_err(|_| "Binary resource namespace")?
            .to_owned();
        if !namespace
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err("Binary resource namespace".into());
        }
        let serial = u64::from_le_bytes(take(&mut input, 8)?.try_into().unwrap());
        let count = u16::from_le_bytes(take(&mut input, 2)?.try_into().unwrap()) as usize;
        if count > MAX_BUFFERS {
            return Err("Binary resource count limit".into());
        }
        let mut store = Self {
            namespace,
            serial,
            bytes: 0,
            entries: BTreeMap::new(),
        };
        for _ in 0..count {
            let generation = u64::from_le_bytes(take(&mut input, 8)?.try_into().unwrap());
            let revision = u64::from_le_bytes(take(&mut input, 8)?.try_into().unwrap());
            let len = u32::from_le_bytes(take(&mut input, 4)?.try_into().unwrap()) as usize;
            if generation == 0
                || generation > revision
                || revision > serial
                || len > MAX_BUFFER
                || store.bytes + len > MAX_BYTES
            {
                return Err("Invalid binary resource entry".into());
            }
            let handle = format!("b/{}/{generation}", &store.namespace[..32]);
            let data: Arc<[u8]> = take(&mut input, len)?.into();
            if store
                .entries
                .insert(handle, Buffer { data, revision })
                .is_some()
            {
                return Err("Duplicate binary resource entry".into());
            }
            store.bytes += len;
        }
        if !input.is_empty() {
            return Err("Trailing binary resource channel bytes".into());
        }
        Ok(store)
    }
    pub fn metadata(&self, handle: &str) -> Result<Metadata, String> {
        let buffer = self
            .entries
            .get(handle)
            .ok_or("Unknown or stale binary handle")?;
        Ok(Metadata {
            version: 1,
            handle: handle.into(),
            size: buffer.data.len(),
            revision: buffer.revision.to_string(),
        })
    }
    pub fn resource_counts(&self) -> (usize, usize) {
        (self.entries.len(), self.bytes)
    }
    pub fn page(&self, handle: &str, offset: usize, count: usize) -> Result<Vec<u8>, String> {
        if count > MAX_PAGE {
            return Err("Binary page limit".into());
        }
        let data = self.bytes(handle)?;
        let end = offset.checked_add(count).ok_or("Binary range overflow")?;
        Ok(data
            .get(offset..end)
            .ok_or("Binary range outside buffer")?
            .to_vec())
    }
    pub fn bytes(&self, handle: &str) -> Result<Arc<[u8]>, String> {
        self.entries
            .get(handle)
            .map(|b| b.data.clone())
            .ok_or_else(|| "Unknown or stale binary handle".into())
    }
    fn next_serial(&self) -> Result<u64, String> {
        self.serial
            .checked_add(1)
            .ok_or_else(|| "Binary generation exhausted".into())
    }
    fn allocate(&mut self, data: Arc<[u8]>) -> Result<Metadata, String> {
        if data.len() > MAX_BUFFER
            || self.entries.len() >= MAX_BUFFERS
            || self.bytes + data.len() > MAX_BYTES
        {
            return Err("Binary resource quota".into());
        }
        let serial = self.next_serial()?;
        let handle = format!("b/{}/{serial}", &self.namespace[..32]);
        self.bytes += data.len();
        self.serial = serial;
        self.entries.insert(
            handle.clone(),
            Buffer {
                data,
                revision: serial,
            },
        );
        self.metadata(&handle)
    }
    pub fn work(&self, operation: &Mutation) -> Result<usize, String> {
        operation.validate()?;
        match operation {
            Mutation::Copy {
                source,
                destination,
                source_offset,
                destination_offset,
                count,
            } => {
                let source_size = self.metadata(source)?.size;
                let dest_size = self.metadata(destination)?.size;
                if source_offset
                    .checked_add(*count)
                    .is_none_or(|end| end > source_size)
                    || destination_offset
                        .checked_add(*count)
                        .is_none_or(|end| end > dest_size)
                {
                    return Err("Binary copy outside buffer".into());
                }
                Ok(dest_size.saturating_add(*count))
            }
            Mutation::Utf8 { text } => Ok(text.len()),
            Mutation::ScalarWrite {
                handle,
                offset,
                scalar,
                endian,
                value,
            } => self.work(&Mutation::Write {
                handle: handle.clone(),
                offset: *offset,
                bytes: scalar.encode(value, *endian)?,
            }),
            Mutation::Create { size } => Ok(*size),
            Mutation::Write {
                handle,
                offset,
                bytes,
            } => {
                let size = self.metadata(handle)?.size;
                if offset.checked_add(bytes.len()).is_none_or(|end| end > size) {
                    return Err("Binary write outside buffer".into());
                }
                Ok(size)
            }
            Mutation::Slice {
                handle,
                offset,
                count,
            } => {
                let size = self.metadata(handle)?.size;
                if offset.checked_add(*count).is_none_or(|end| end > size) {
                    return Err("Binary slice outside buffer".into());
                }
                Ok(*count)
            }
            Mutation::Concat { handles } => {
                let mut size = 0usize;
                for handle in handles {
                    size = size
                        .checked_add(self.metadata(handle)?.size)
                        .ok_or("Binary concat overflow")?;
                }
                if size > MAX_BUFFER {
                    return Err("Binary concatenation limit".into());
                }
                Ok(size)
            }
            Mutation::Release { handle } => {
                self.metadata(handle)?;
                Ok(0)
            }
        }
    }
    pub fn apply(&mut self, operation: &Mutation) -> Result<Option<Metadata>, String> {
        self.work(operation)?;
        match operation {
            Mutation::Copy {
                source,
                destination,
                source_offset,
                destination_offset,
                count,
            } => {
                let source_data = self.bytes(source)?;
                let serial = self.next_serial()?;
                let dest = self
                    .entries
                    .get_mut(destination)
                    .ok_or("Unknown binary destination")?;
                Arc::make_mut(&mut dest.data)[*destination_offset..*destination_offset + *count]
                    .copy_from_slice(&source_data[*source_offset..*source_offset + *count]);
                dest.revision = serial;
                self.serial = serial;
                self.metadata(destination).map(Some)
            }
            Mutation::Utf8 { text } => self.allocate(text.as_bytes().into()).map(Some),
            Mutation::ScalarWrite {
                handle,
                offset,
                scalar,
                endian,
                value,
            } => self.apply(&Mutation::Write {
                handle: handle.clone(),
                offset: *offset,
                bytes: scalar.encode(value, *endian)?,
            }),
            Mutation::Create { size } => self.allocate(vec![0; *size].into()).map(Some),
            Mutation::Write {
                handle,
                offset,
                bytes,
            } => {
                let serial = self.next_serial()?;
                let buffer = self
                    .entries
                    .get_mut(handle)
                    .ok_or("Unknown binary handle")?;
                Arc::make_mut(&mut buffer.data)[*offset..*offset + bytes.len()]
                    .copy_from_slice(bytes);
                buffer.revision = serial;
                self.serial = serial;
                self.metadata(handle).map(Some)
            }
            Mutation::Slice {
                handle,
                offset,
                count,
            } => {
                let data = self.bytes(handle)?;
                self.allocate(data[*offset..*offset + *count].to_vec().into())
                    .map(Some)
            }
            Mutation::Concat { handles } => {
                let mut data = Vec::with_capacity(self.work(operation)?);
                for handle in handles {
                    data.extend_from_slice(&self.bytes(handle)?);
                }
                self.allocate(data.into()).map(Some)
            }
            Mutation::Release { handle } => {
                let removed = self.entries.remove(handle).ok_or("Unknown binary handle")?;
                self.bytes -= removed.data.len();
                Ok(None)
            }
        }
    }
}
