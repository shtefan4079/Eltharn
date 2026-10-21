#![forbid(unsafe_code)]

pub mod byte_buf;
pub mod frame;
pub mod packets;
pub mod var_number;

pub use byte_buf::{BufferError, PacketBuffer};
pub use frame::{FrameError, read_frame, write_frame};
pub use var_number::{
    DecodeError, decode_var_i32, decode_var_i64, encode_var_i32, encode_var_i64, var_i32_size,
    var_i64_size,
};

pub trait Packet: Sized {
    const ID: i32;

    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError>;
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError>;
}

pub trait PacketDecode: Packet {
    fn decode_from_bytes(bytes: Vec<u8>) -> Result<Self, BufferError> {
        let mut buffer = PacketBuffer::from_bytes(bytes);
        Self::decode(&mut buffer)
    }
}

impl<T: Packet> PacketDecode for T {}
