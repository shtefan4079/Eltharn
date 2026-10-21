use crate::var_number::{
    DecodeError, decode_var_i32, decode_var_i64, encode_var_i32, encode_var_i64,
};
use el_nbt::Tag;
use el_nbt::io::{read_unnamed_tag, write_payload};
use std::{fmt, io::Cursor};
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BufferError {
    VarNumber(DecodeError),
    NotEnoughBytes { expected: usize, remaining: usize },
    NegativeLength,
    StringTooLong { actual: usize, max: usize },
    EncodedStringTooLong { actual: usize, max: usize },
    CollectionTooLong { actual: usize, max: usize },
    InvalidUtf8,
    InvalidEnumId { name: &'static str, id: i32 },
    Nbt(String),
}
impl fmt::Display for BufferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::VarNumber(error) => {
                write!(formatter, "packet buffer var number: {error}")
            }
            Self::NotEnoughBytes {
                expected,
                remaining,
            } => {
                write!(
                    formatter,
                    "packet buffer needs {expected} bytes but has {remaining}"
                )
            }
            Self::NegativeLength => formatter.write_str("packet buffer has a negative length"),
            Self::StringTooLong { actual, max } => {
                write!(formatter, "string of {actual} chars exceeds limit {max}")
            }
            Self::EncodedStringTooLong { actual, max } => {
                write!(
                    formatter,
                    "encoded string of {actual} bytes exceeds limit {max}"
                )
            }
            Self::CollectionTooLong { actual, max } => {
                write!(
                    formatter,
                    "collection of {actual} items exceeds limit {max}"
                )
            }
            Self::InvalidUtf8 => formatter.write_str("packet buffer holds invalid UTF-8"),
            Self::InvalidEnumId { name, id } => {
                write!(formatter, "unknown {name} id: {id}")
            }
            Self::Nbt(detail) => write!(formatter, "packet buffer NBT error: {detail}"),
        }
    }
}
impl std::error::Error for BufferError {}
impl From<DecodeError> for BufferError {
    fn from(value: DecodeError) -> Self {
        BufferError::VarNumber(value)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PacketBuffer {
    data: Vec<u8>,
    reader_index: usize,
}
impl PacketBuffer {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self {
            data,
            reader_index: 0,
        }
    }
    pub fn into_inner(self) -> Vec<u8> {
        self.data
    }
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.reader_index)
    }
    pub fn write_u8(&mut self, value: u8) {
        self.data.push(value);
    }
    pub fn read_u8(&mut self) -> Result<u8, BufferError> {
        if self.remaining() < 1 {
            return Err(BufferError::NotEnoughBytes {
                expected: 1,
                remaining: self.remaining(),
            });
        }
        let value = self.data[self.reader_index];
        self.reader_index += 1;
        Ok(value)
    }
    pub fn write_bool(&mut self, value: bool) {
        self.write_u8(if value { 1 } else { 0 });
    }
    pub fn read_bool(&mut self) -> Result<bool, BufferError> {
        Ok(self.read_u8()? != 0)
    }
    pub fn write_i8(&mut self, value: i8) {
        self.write_u8(value as u8);
    }
    pub fn read_i8(&mut self) -> Result<i8, BufferError> {
        Ok(self.read_u8()? as i8)
    }
    pub fn write_uuid_bytes(&mut self, value: &[u8; 16]) {
        self.write_bytes(value);
    }
    pub fn read_uuid_bytes(&mut self) -> Result<[u8; 16], BufferError> {
        let bytes = self.read_bytes(16)?;
        Ok(bytes
            .try_into()
            .expect("read_bytes(16) must return exactly 16 bytes"))
    }
    pub fn write_u16(&mut self, value: u16) {
        self.write_bytes(&value.to_be_bytes());
    }
    pub fn read_u16(&mut self) -> Result<u16, BufferError> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
    }
    pub fn write_i16(&mut self, value: i16) {
        self.write_bytes(&value.to_be_bytes());
    }
    pub fn read_i16(&mut self) -> Result<i16, BufferError> {
        let bytes = self.read_bytes(2)?;
        Ok(i16::from_be_bytes([bytes[0], bytes[1]]))
    }
    pub fn write_i32(&mut self, value: i32) {
        self.write_bytes(&value.to_be_bytes());
    }
    pub fn read_i32(&mut self) -> Result<i32, BufferError> {
        let bytes = self.read_bytes(4)?;
        Ok(i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
    pub fn write_i64(&mut self, value: i64) {
        self.write_bytes(&value.to_be_bytes());
    }
    pub fn read_i64(&mut self) -> Result<i64, BufferError> {
        let bytes = self.read_bytes(8)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
    pub fn write_f32(&mut self, value: f32) {
        self.write_bytes(&value.to_be_bytes());
    }
    pub fn read_f32(&mut self) -> Result<f32, BufferError> {
        let bytes = self.read_bytes(4)?;
        Ok(f32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }
    pub fn write_f64(&mut self, value: f64) {
        self.write_bytes(&value.to_be_bytes());
    }
    pub fn read_f64(&mut self) -> Result<f64, BufferError> {
        let bytes = self.read_bytes(8)?;
        Ok(f64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }
    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.data.extend_from_slice(bytes);
    }
    pub fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>, BufferError> {
        if self.remaining() < len {
            return Err(BufferError::NotEnoughBytes {
                expected: len,
                remaining: self.remaining(),
            });
        }
        let start = self.reader_index;
        self.reader_index += len;
        Ok(self.data[start..self.reader_index].to_vec())
    }
    pub fn write_nbt_tag(&mut self, tag: Option<&Tag>) -> Result<(), BufferError> {
        let Some(tag) = tag else {
            self.write_u8(Tag::End.get_id());
            return Ok(());
        };
        self.write_u8(tag.get_id());
        let mut payload = Vec::new();
        write_payload(&mut payload, tag).map_err(|error| BufferError::Nbt(error.to_string()))?;
        self.write_bytes(&payload);
        Ok(())
    }
    pub fn read_nbt_tag(&mut self) -> Result<Option<Tag>, BufferError> {
        let remaining = &self.data[self.reader_index..];
        if remaining.is_empty() {
            return Err(BufferError::NotEnoughBytes {
                expected: 1,
                remaining: 0,
            });
        }
        let mut cursor = Cursor::new(remaining);
        let tag =
            read_unnamed_tag(&mut cursor).map_err(|error| BufferError::Nbt(error.to_string()))?;
        self.reader_index += cursor.position() as usize;
        Ok(tag)
    }
    pub fn write_var_i32(&mut self, value: i32) {
        self.write_bytes(&encode_var_i32(value));
    }
    pub fn read_var_i32(&mut self) -> Result<i32, BufferError> {
        let (value, bytes_read) = decode_var_i32(&self.data[self.reader_index..])?;
        self.reader_index += bytes_read;
        Ok(value)
    }
    pub fn write_var_i64(&mut self, value: i64) {
        self.write_bytes(&encode_var_i64(value));
    }
    pub fn read_var_i64(&mut self) -> Result<i64, BufferError> {
        let (value, bytes_read) = decode_var_i64(&self.data[self.reader_index..])?;
        self.reader_index += bytes_read;
        Ok(value)
    }
    pub fn write_utf(&mut self, value: &str, max_length: usize) -> Result<(), BufferError> {
        let java_len = java_string_len(value);
        if java_len > max_length {
            return Err(BufferError::StringTooLong {
                actual: java_len,
                max: max_length,
            });
        }
        let max_encoded_len = max_utf8_bytes(max_length);
        let bytes = value.as_bytes();
        if bytes.len() > max_encoded_len {
            return Err(BufferError::EncodedStringTooLong {
                actual: bytes.len(),
                max: max_encoded_len,
            });
        }
        self.write_var_i32(bytes.len() as i32);
        self.write_bytes(bytes);
        Ok(())
    }
    pub fn read_utf(&mut self, max_length: usize) -> Result<String, BufferError> {
        let max_encoded_len = max_utf8_bytes(max_length);
        let len = self.read_var_i32()?;
        if len < 0 {
            return Err(BufferError::NegativeLength);
        }
        let len = len as usize;
        if len > max_encoded_len {
            return Err(BufferError::EncodedStringTooLong {
                actual: len,
                max: max_encoded_len,
            });
        }
        let bytes = self.read_bytes(len)?;
        let value = String::from_utf8(bytes).map_err(|_| BufferError::InvalidUtf8)?;
        let java_len = java_string_len(&value);
        if java_len > max_length {
            return Err(BufferError::StringTooLong {
                actual: java_len,
                max: max_length,
            });
        }
        Ok(value)
    }
}
fn java_string_len(value: &str) -> usize {
    value.encode_utf16().count()
}
fn max_utf8_bytes(max_length: usize) -> usize {
    max_length.saturating_mul(3)
}
