use std::fmt;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    TooBig,
    Incomplete,
}
impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooBig => formatter.write_str("var number exceeds its maximum byte length"),
            Self::Incomplete => formatter.write_str("var number input ended early"),
        }
    }
}
impl std::error::Error for DecodeError {}
pub fn var_i32_size(value: i32) -> usize {
    let value = value as u32;
    for size in 1..5 {
        if (value & (!0u32 << (size * 7))) == 0 {
            return size;
        }
    }
    5
}
pub fn encode_var_i32(value: i32) -> Vec<u8> {
    let mut value = value as u32;
    let mut output = Vec::with_capacity(5);
    while (value & !0x7f) != 0 {
        output.push(((value & 0x7f) | 0x80) as u8);
        value >>= 7;
    }
    output.push(value as u8);
    output
}
pub fn decode_var_i32(input: &[u8]) -> Result<(i32, usize), DecodeError> {
    let mut output = 0u32;
    for (index, byte) in input.iter().copied().enumerate() {
        if index >= 5 {
            return Err(DecodeError::TooBig);
        }
        let value = (byte & 0x7f) as u32;
        output |= value << (index * 7);
        if (byte & 0x80) == 0 {
            return Ok((output as i32, index + 1));
        }
    }
    Err(DecodeError::Incomplete)
}
pub fn var_i64_size(value: i64) -> usize {
    let value = value as u64;
    for size in 1..10 {
        if (value & (!0u64 << (size * 7))) == 0 {
            return size;
        }
    }
    10
}
pub fn encode_var_i64(value: i64) -> Vec<u8> {
    let mut value = value as u64;
    let mut output = Vec::with_capacity(10);
    while (value & !0x7f) != 0 {
        output.push(((value & 0x7f) | 0x80) as u8);
        value >>= 7;
    }
    output.push(value as u8);
    output
}
pub fn decode_var_i64(input: &[u8]) -> Result<(i64, usize), DecodeError> {
    let mut output = 0u64;
    for (index, byte) in input.iter().copied().enumerate() {
        if index >= 10 {
            return Err(DecodeError::TooBig);
        }
        let value = (byte & 0x7f) as u64;
        output |= value << (index * 7);
        if (byte & 0x80) == 0 {
            return Ok((output as i64, index + 1));
        }
    }
    Err(DecodeError::Incomplete)
}
