use crate::{BufferError, Packet, PacketBuffer};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServerboundStatusRequestPacket;
impl Packet for ServerboundStatusRequestPacket {
    const ID: i32 = 0;
    fn encode(&self, _output: &mut PacketBuffer) -> Result<(), BufferError> {
        Ok(())
    }
    fn decode(_input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundStatusResponsePacket {
    pub json: String,
}
impl Packet for ClientboundStatusResponsePacket {
    const ID: i32 = 0;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_utf(&self.json, 32767)
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            json: input.read_utf(32767)?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ServerboundPingRequestPacket {
    pub time: i64,
}
impl Packet for ServerboundPingRequestPacket {
    const ID: i32 = 1;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i64(self.time);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            time: input.read_i64()?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientboundPongResponsePacket {
    pub time: i64,
}
impl Packet for ClientboundPongResponsePacket {
    const ID: i32 = 1;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i64(self.time);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            time: input.read_i64()?,
        })
    }
}
