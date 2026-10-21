use crate::{BufferError, Packet, PacketBuffer};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientIntent {
    Status,
    Login,
    Transfer,
}
impl ClientIntent {
    pub fn id(self) -> i32 {
        match self {
            ClientIntent::Status => 1,
            ClientIntent::Login => 2,
            ClientIntent::Transfer => 3,
        }
    }
    pub fn by_id(id: i32) -> Result<Self, BufferError> {
        match id {
            1 => Ok(ClientIntent::Status),
            2 => Ok(ClientIntent::Login),
            3 => Ok(ClientIntent::Transfer),
            _ => Err(BufferError::InvalidEnumId {
                name: "ClientIntent",
                id,
            }),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientIntentionPacket {
    pub protocol_version: i32,
    pub host_name: String,
    pub port: u16,
    pub intention: ClientIntent,
}
impl Packet for ClientIntentionPacket {
    const ID: i32 = 0;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.protocol_version);
        output.write_utf(&self.host_name, 32767)?;
        output.write_u16(self.port);
        output.write_var_i32(self.intention.id());
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let protocol_version = input.read_var_i32()?;
        let host_name = input.read_utf(255)?;
        let port = input.read_u16()?;
        let intention = ClientIntent::by_id(input.read_var_i32()?)?;
        Ok(Self {
            protocol_version,
            host_name,
            port,
            intention,
        })
    }
}
