use crate::io::{read_named_tag, write_named_tag};
use crate::tag::Tag;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use flate2::Compression;
use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::ZlibEncoder;
use std::io::{self, Read, Seek, SeekFrom, Write};

pub struct RegionFile<S> {
    stream: S,
    offsets: [u32; 1024],
    timestamps: [u32; 1024],
}

impl<S: Read + Write + Seek> RegionFile<S> {
    pub fn new(mut stream: S) -> io::Result<Self> {
        let mut offsets = [0u32; 1024];
        let mut timestamps = [0u32; 1024];

        let size = stream.seek(SeekFrom::End(0))?;
        if size < 8192 {
            stream.seek(SeekFrom::Start(0))?;
            let zeros = vec![0u8; 8192 - size as usize];
            stream.write_all(&zeros)?;
        }

        stream.seek(SeekFrom::Start(0))?;
        for i in 0..1024 {
            offsets[i] = stream.read_u32::<BigEndian>()?;
        }
        for i in 0..1024 {
            timestamps[i] = stream.read_u32::<BigEndian>()?;
        }

        Ok(RegionFile {
            stream,
            offsets,
            timestamps,
        })
    }

    fn get_chunk_index(x: i32, z: i32) -> usize {
        ((x & 31) + (z & 31) * 32) as usize
    }

    pub fn read_chunk(&mut self, x: i32, z: i32) -> io::Result<Option<Tag>> {
        let idx = Self::get_chunk_index(x, z);
        let offset_val = self.offsets[idx];
        if offset_val == 0 {
            return Ok(None);
        }

        let sector_offset = (offset_val >> 8) as u64;
        let sector_count = (offset_val & 0xFF) as u64;

        self.stream.seek(SeekFrom::Start(sector_offset * 4096))?;
        let length = self.stream.read_u32::<BigEndian>()?;
        let compression_type = self.stream.read_u8()?;

        if length == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Zero-length chunk",
            ));
        }

        let stream_len = (length - 1) as u64;
        let sector_capacity = sector_count.saturating_mul(4096).saturating_sub(5);
        let data_start = self.stream.stream_position()?;
        let file_len = self.stream.seek(SeekFrom::End(0))?;
        self.stream.seek(SeekFrom::Start(data_start))?;
        let remaining_file_bytes = file_len.saturating_sub(data_start);
        if stream_len > sector_capacity || stream_len > remaining_file_bytes {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Chunk stream length {stream_len} exceeds its allocated sectors \
                     ({sector_capacity} bytes) or the remaining file ({remaining_file_bytes} bytes)"
                ),
            ));
        }

        let mut compressed_data = vec![0; stream_len as usize];
        self.stream.read_exact(&mut compressed_data)?;

        let (_, tag) = match compression_type {
            1 => {
                let mut decoder = GzDecoder::new(&compressed_data[..]);
                read_named_tag(&mut decoder)?
            }
            2 => {
                let mut decoder = ZlibDecoder::new(&compressed_data[..]);
                read_named_tag(&mut decoder)?
            }
            3 => {
                let mut cursor = io::Cursor::new(compressed_data);
                read_named_tag(&mut cursor)?
            }
            t => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Unknown compression type: {}", t),
                ));
            }
        };

        Ok(Some(tag))
    }

    pub fn write_chunk(&mut self, x: i32, z: i32, tag: &Tag) -> io::Result<()> {
        let idx = Self::get_chunk_index(x, z);

        let mut compressed_buf = Vec::new();
        {
            let mut encoder = ZlibEncoder::new(&mut compressed_buf, Compression::default());
            write_named_tag(&mut encoder, "", tag)?;
            encoder.finish()?;
        }

        let payload_len = compressed_buf.len() + 1;
        let total_bytes_needed = payload_len + 4;
        let sectors_needed = ((total_bytes_needed + 4095) / 4096) as u32;

        if sectors_needed >= 256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Chunk data too large for region sector count",
            ));
        }

        let mut sector_alloc = vec![false; 1024 * 1024];
        sector_alloc[0] = true;
        sector_alloc[1] = true;

        for i in 0..1024 {
            if i != idx {
                let off = self.offsets[i];
                if off != 0 {
                    let sec_start = (off >> 8) as usize;
                    let sec_count = (off & 0xFF) as usize;
                    for s in sec_start..(sec_start + sec_count) {
                        if s < sector_alloc.len() {
                            sector_alloc[s] = true;
                        }
                    }
                }
            }
        }

        let mut target_sector = 2;
        let mut run = 0;
        let mut found = false;
        for s in 2..sector_alloc.len() {
            if !sector_alloc[s] {
                run += 1;
                if run == sectors_needed {
                    target_sector = (s - run as usize + 1) as u32;
                    found = true;
                    break;
                }
            } else {
                run = 0;
            }
        }

        if !found {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "No contiguous free sector run available for chunk data",
            ));
        }

        self.stream
            .seek(SeekFrom::Start(target_sector as u64 * 4096))?;
        self.stream.write_u32::<BigEndian>(payload_len as u32)?;
        self.stream.write_u8(2)?;
        self.stream.write_all(&compressed_buf)?;

        let bytes_written = 5 + compressed_buf.len();
        let padding_needed = (sectors_needed as usize * 4096) - bytes_written;
        if padding_needed > 0 {
            let padding = vec![0u8; padding_needed];
            self.stream.write_all(&padding)?;
        }

        let new_offset = (target_sector << 8) | (sectors_needed & 0xFF);
        self.offsets[idx] = new_offset;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;
        self.timestamps[idx] = timestamp;

        self.stream.seek(SeekFrom::Start(0))?;
        for i in 0..1024 {
            self.stream.write_u32::<BigEndian>(self.offsets[i])?;
        }
        for i in 0..1024 {
            self.stream.write_u32::<BigEndian>(self.timestamps[i])?;
        }

        self.stream.flush()?;
        Ok(())
    }
}
