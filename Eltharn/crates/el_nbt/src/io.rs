use crate::tag::Tag;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flate2::Compression;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::{GzEncoder, ZlibEncoder};
use std::io::{self, Read, Write};
pub const MAX_ALLOC_BYTES: usize = 64 * 1024 * 1024;
fn check_alloc_len<T>(len: usize, what: &str) -> io::Result<()> {
    let per_item = std::mem::size_of::<T>().max(1);
    if len > MAX_ALLOC_BYTES / per_item {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{what} length {len} exceeds the {MAX_ALLOC_BYTES}-byte allocation cap"),
        ));
    }
    Ok(())
}
fn read_exact_bounded<R: Read>(reader: &mut R, len: usize) -> io::Result<Vec<u8>> {
    const STEP: usize = 8 * 1024;
    let mut buf: Vec<u8> = Vec::new();
    let mut remaining = len;
    while remaining > 0 {
        let step = remaining.min(STEP);
        let start = buf.len();
        buf.try_reserve(step).map_err(|_| {
            io::Error::new(io::ErrorKind::OutOfMemory, "NBT buffer allocation failed")
        })?;
        buf.resize(start + step, 0);
        reader.read_exact(&mut buf[start..])?;
        remaining -= step;
    }
    Ok(buf)
}
pub fn read_named_tag<R: Read>(reader: &mut R) -> io::Result<(String, Tag)> {
    read_named_tag_with_depth(reader, 0)
}
pub fn read_unnamed_tag<R: Read>(reader: &mut R) -> io::Result<Option<Tag>> {
    let tag_type = reader.read_u8()?;
    if tag_type == Tag::End.get_id() {
        return Ok(None);
    }
    Ok(Some(read_payload_with_depth(reader, tag_type, 0)?))
}
fn read_named_tag_with_depth<R: Read>(reader: &mut R, depth: usize) -> io::Result<(String, Tag)> {
    let tag_type = reader.read_u8()?;
    if tag_type == 0 {
        return Ok(("".to_string(), Tag::End));
    }
    let name = read_string(reader)?;
    let tag = read_payload_with_depth(reader, tag_type, depth)?;
    Ok((name, tag))
}
pub fn read_payload<R: Read>(reader: &mut R, tag_type: u8) -> io::Result<Tag> {
    read_payload_with_depth(reader, tag_type, 0)
}
fn read_payload_with_depth<R: Read>(reader: &mut R, tag_type: u8, depth: usize) -> io::Result<Tag> {
    if depth > 512 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "NBT nesting depth limit exceeded",
        ));
    }
    match tag_type {
        0 => Ok(Tag::End),
        1 => Ok(Tag::Byte(reader.read_i8()?)),
        2 => Ok(Tag::Short(reader.read_i16::<BigEndian>()?)),
        3 => Ok(Tag::Int(reader.read_i32::<BigEndian>()?)),
        4 => Ok(Tag::Long(reader.read_i64::<BigEndian>()?)),
        5 => Ok(Tag::Float(reader.read_f32::<BigEndian>()?)),
        6 => Ok(Tag::Double(reader.read_f64::<BigEndian>()?)),
        7 => {
            let len = reader.read_i32::<BigEndian>()?;
            if len < 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Negative ByteArray length",
                ));
            }
            check_alloc_len::<i8>(len as usize, "ByteArray")?;
            let buf = read_exact_bounded(reader, len as usize)?;
            let ibuf = unsafe { std::mem::transmute::<Vec<u8>, Vec<i8>>(buf) };
            Ok(Tag::ByteArray(ibuf))
        }
        8 => Ok(Tag::String(read_string(reader)?)),
        9 => {
            let item_type = reader.read_u8()?;
            let len = reader.read_i32::<BigEndian>()?;
            if len < 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Negative List length",
                ));
            }
            check_alloc_len::<Tag>(len as usize, "List")?;
            let mut items = Vec::new();
            for _ in 0..len {
                items.push(read_payload_with_depth(reader, item_type, depth + 1)?);
            }
            Ok(Tag::List(items))
        }
        10 => {
            let mut fields: Vec<(String, Tag)> = Vec::new();
            loop {
                let (name, tag) = read_named_tag_with_depth(reader, depth + 1)?;
                if tag == Tag::End {
                    break;
                }
                fields.push((name, tag));
            }
            Ok(Tag::Compound(fields))
        }
        11 => {
            let len = reader.read_i32::<BigEndian>()?;
            if len < 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Negative IntArray length",
                ));
            }
            check_alloc_len::<i32>(len as usize, "IntArray")?;
            let mut items = Vec::new();
            for _ in 0..len {
                items.push(reader.read_i32::<BigEndian>()?);
            }
            Ok(Tag::IntArray(items))
        }
        12 => {
            let len = reader.read_i32::<BigEndian>()?;
            if len < 0 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Negative LongArray length",
                ));
            }
            check_alloc_len::<i64>(len as usize, "LongArray")?;
            let mut items = Vec::new();
            for _ in 0..len {
                items.push(reader.read_i64::<BigEndian>()?);
            }
            Ok(Tag::LongArray(items))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid tag type: {}", tag_type),
        )),
    }
}
fn read_string<R: Read>(reader: &mut R) -> io::Result<String> {
    let len = reader.read_u16::<BigEndian>()?;
    let mut buf = vec![0; len as usize];
    reader.read_exact(&mut buf)?;
    decode_modified_utf8(&buf).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
fn decode_modified_utf8(bytes: &[u8]) -> Result<String, String> {
    let mut out = String::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let (unit, next) = decode_modified_utf8_unit(bytes, i)?;
        if (0xD800..=0xDBFF).contains(&unit) {
            if next < bytes.len() {
                let (low, after_low) = decode_modified_utf8_unit(bytes, next)?;
                if (0xDC00..=0xDFFF).contains(&low) {
                    let code_point =
                        0x10000 + (((unit - 0xD800) as u32) << 10) + (low - 0xDC00) as u32;
                    out.push(
                        char::from_u32(code_point).ok_or_else(|| {
                            "invalid surrogate pair in modified UTF-8".to_string()
                        })?,
                    );
                    i = after_low;
                    continue;
                }
            }
            return Err("unpaired high surrogate in modified UTF-8".to_string());
        }
        if (0xDC00..=0xDFFF).contains(&unit) {
            return Err("unpaired low surrogate in modified UTF-8".to_string());
        }
        out.push(
            char::from_u32(unit as u32)
                .ok_or_else(|| "invalid code unit in modified UTF-8".to_string())?,
        );
        i = next;
    }
    Ok(out)
}
fn decode_modified_utf8_unit(bytes: &[u8], i: usize) -> Result<(u16, usize), String> {
    let b0 = bytes[i];
    if b0 & 0x80 == 0 {
        return Ok((b0 as u16, i + 1));
    }
    if b0 & 0xE0 == 0xC0 {
        let b1 = *bytes
            .get(i + 1)
            .ok_or_else(|| "truncated two-byte modified-UTF-8 sequence".to_string())?;
        if b1 & 0xC0 != 0x80 {
            return Err("malformed continuation byte in modified UTF-8".to_string());
        }
        return Ok(((b0 as u16 & 0x1F) << 6 | (b1 as u16 & 0x3F), i + 2));
    }
    if b0 & 0xF0 == 0xE0 {
        let b1 = *bytes
            .get(i + 1)
            .ok_or_else(|| "truncated three-byte modified-UTF-8 sequence".to_string())?;
        let b2 = *bytes
            .get(i + 2)
            .ok_or_else(|| "truncated three-byte modified-UTF-8 sequence".to_string())?;
        if b1 & 0xC0 != 0x80 || b2 & 0xC0 != 0x80 {
            return Err("malformed continuation byte in modified UTF-8".to_string());
        }
        return Ok((
            (b0 as u16 & 0x0F) << 12 | (b1 as u16 & 0x3F) << 6 | (b2 as u16 & 0x3F),
            i + 3,
        ));
    }
    Err("malformed leading byte in modified UTF-8".to_string())
}
pub fn write_named_tag<W: Write>(writer: &mut W, name: &str, tag: &Tag) -> io::Result<()> {
    writer.write_u8(tag.get_id())?;
    if *tag == Tag::End {
        return Ok(());
    }
    write_string(writer, name)?;
    write_payload(writer, tag)
}
pub fn write_payload<W: Write>(writer: &mut W, tag: &Tag) -> io::Result<()> {
    match tag {
        Tag::End => Ok(()),
        Tag::Byte(v) => writer.write_i8(*v),
        Tag::Short(v) => writer.write_i16::<BigEndian>(*v),
        Tag::Int(v) => writer.write_i32::<BigEndian>(*v),
        Tag::Long(v) => writer.write_i64::<BigEndian>(*v),
        Tag::Float(v) => writer.write_f32::<BigEndian>(*v),
        Tag::Double(v) => writer.write_f64::<BigEndian>(*v),
        Tag::ByteArray(v) => {
            writer.write_i32::<BigEndian>(v.len() as i32)?;
            let ubuf = unsafe { std::slice::from_raw_parts(v.as_ptr() as *const u8, v.len()) };
            writer.write_all(ubuf)
        }
        Tag::String(v) => write_string(writer, v),
        Tag::List(v) => {
            let item_type = if v.is_empty() { 0 } else { v[0].get_id() };
            writer.write_u8(item_type)?;
            writer.write_i32::<BigEndian>(v.len() as i32)?;
            for item in v {
                write_payload(writer, item)?;
            }
            Ok(())
        }
        Tag::Compound(fields) => {
            for (name, item) in fields {
                write_named_tag(writer, name, item)?;
            }
            writer.write_u8(Tag::End.get_id())
        }
        Tag::IntArray(v) => {
            writer.write_i32::<BigEndian>(v.len() as i32)?;
            for item in v {
                writer.write_i32::<BigEndian>(*item)?;
            }
            Ok(())
        }
        Tag::LongArray(v) => {
            writer.write_i32::<BigEndian>(v.len() as i32)?;
            for item in v {
                writer.write_i64::<BigEndian>(*item)?;
            }
            Ok(())
        }
    }
}
fn write_string<W: Write>(writer: &mut W, s: &str) -> io::Result<()> {
    let mut bytes = Vec::with_capacity(s.len());
    for c in s.chars() {
        let code = c as u32;
        if code == 0 {
            bytes.push(0xC0);
            bytes.push(0x80);
        } else if code < 0x80 {
            bytes.push(code as u8);
        } else if code < 0x800 {
            bytes.push(0xC0 | (code >> 6) as u8);
            bytes.push(0x80 | (code & 0x3F) as u8);
        } else if code < 0x10000 {
            push_three_byte_unit(&mut bytes, code as u16);
        } else {
            let value = code - 0x10000;
            push_three_byte_unit(&mut bytes, 0xD800 | (value >> 10) as u16);
            push_three_byte_unit(&mut bytes, 0xDC00 | (value & 0x3FF) as u16);
        }
    }
    if bytes.len() > u16::MAX as usize {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "String length exceeds u16::MAX",
        ));
    }
    writer.write_u16::<BigEndian>(bytes.len() as u16)?;
    writer.write_all(&bytes)
}
fn push_three_byte_unit(bytes: &mut Vec<u8>, unit: u16) {
    bytes.push(0xE0 | (unit >> 12) as u8);
    bytes.push(0x80 | ((unit >> 6) & 0x3F) as u8);
    bytes.push(0x80 | (unit & 0x3F) as u8);
}
pub fn read_gzip<R: Read>(reader: R) -> io::Result<(String, Tag)> {
    let mut decoder = GzDecoder::new(reader);
    read_named_tag(&mut decoder)
}
pub fn read_zlib<R: Read>(reader: R) -> io::Result<(String, Tag)> {
    let mut decoder = ZlibDecoder::new(reader);
    read_named_tag(&mut decoder)
}
pub fn write_gzip<W: Write>(writer: W, name: &str, tag: &Tag) -> io::Result<()> {
    let mut encoder = GzEncoder::new(writer, Compression::default());
    write_named_tag(&mut encoder, name, tag)?;
    encoder.finish()?;
    Ok(())
}
pub fn write_zlib<W: Write>(writer: W, name: &str, tag: &Tag) -> io::Result<()> {
    let mut encoder = ZlibEncoder::new(writer, Compression::default());
    write_named_tag(&mut encoder, name, tag)?;
    encoder.finish()?;
    Ok(())
}
pub fn read_any_compound(bytes: &[u8]) -> io::Result<Tag> {
    if bytes.is_empty() {
        return Ok(Tag::Compound(Vec::new()));
    }
    let mut cursor = io::Cursor::new(bytes);
    if let Ok((_name, Tag::Compound(fields))) = read_named_tag(&mut cursor) {
        if !fields.is_empty() {
            return Ok(Tag::Compound(fields));
        }
    }
    let mut cursor = io::Cursor::new(bytes);
    if let Ok(Some(Tag::Compound(fields))) = read_unnamed_tag(&mut cursor) {
        if !fields.is_empty() {
            return Ok(Tag::Compound(fields));
        }
    }
    let mut cursor = io::Cursor::new(bytes);
    if let Ok(Tag::Compound(fields)) = read_payload(&mut cursor, 10) {
        if !fields.is_empty() {
            return Ok(Tag::Compound(fields));
        }
    }
    let cursor = io::Cursor::new(bytes);
    if let Ok((_name, Tag::Compound(fields))) = read_zlib(cursor) {
        return Ok(Tag::Compound(fields));
    }
    let cursor = io::Cursor::new(bytes);
    if let Ok((_name, Tag::Compound(fields))) = read_gzip(cursor) {
        return Ok(Tag::Compound(fields));
    }
    let mut cursor = io::Cursor::new(bytes);
    if let Ok((_name, Tag::Compound(fields))) = read_named_tag(&mut cursor) {
        return Ok(Tag::Compound(fields));
    }
    let mut cursor = io::Cursor::new(bytes);
    if let Ok(Some(Tag::Compound(fields))) = read_unnamed_tag(&mut cursor) {
        return Ok(Tag::Compound(fields));
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Failed to decode NBT compound",
    ))
}
