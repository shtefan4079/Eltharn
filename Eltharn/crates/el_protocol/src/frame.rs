use crate::var_number::{DecodeError, encode_var_i32};
use std::{
    fmt,
    io::{self, Read, Write},
};
#[derive(Debug)]
pub enum FrameError {
    Io(io::Error),
    VarInt(DecodeError),
    NegativeLength(i32),
    TooLarge { length: usize, max: usize },
}
impl fmt::Display for FrameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "frame IO error: {error}"),
            Self::VarInt(error) => write!(formatter, "frame length prefix: {error}"),
            Self::NegativeLength(length) => {
                write!(formatter, "frame has a negative length: {length}")
            }
            Self::TooLarge { length, max } => {
                write!(formatter, "frame length {length} exceeds limit {max}")
            }
        }
    }
}
impl std::error::Error for FrameError {}
impl From<io::Error> for FrameError {
    fn from(value: io::Error) -> Self {
        FrameError::Io(value)
    }
}
impl From<DecodeError> for FrameError {
    fn from(value: DecodeError) -> Self {
        FrameError::VarInt(value)
    }
}
pub fn read_frame<R: Read>(reader: &mut R, max_len: usize) -> Result<Vec<u8>, FrameError> {
    let length = read_var_i32(reader)?;
    if length < 0 {
        return Err(FrameError::NegativeLength(length));
    }
    let length = length as usize;
    if length > max_len {
        return Err(FrameError::TooLarge {
            length,
            max: max_len,
        });
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload)?;
    Ok(payload)
}
pub fn write_frame<W: Write>(writer: &mut W, payload: &[u8]) -> Result<(), FrameError> {
    writer.write_all(&encode_var_i32(payload.len() as i32))?;
    writer.write_all(payload)?;
    Ok(())
}
fn read_var_i32<R: Read>(reader: &mut R) -> Result<i32, FrameError> {
    let mut output = 0u32;
    for index in 0..5 {
        let mut byte = [0u8; 1];
        reader.read_exact(&mut byte)?;
        let byte = byte[0];
        output |= ((byte & 0x7f) as u32) << (index * 7);
        if (byte & 0x80) == 0 {
            return Ok(output as i32);
        }
    }
    Err(FrameError::VarInt(DecodeError::TooBig))
}
