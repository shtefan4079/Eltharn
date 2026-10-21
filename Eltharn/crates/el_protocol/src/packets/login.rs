use crate::{BufferError, Packet, PacketBuffer};
const MAX_LOGIN_NAME_LENGTH: usize = 16;
const MAX_DISCONNECT_JSON_LENGTH: usize = 262_144;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundHelloPacket {
    pub name: String,
    pub profile_id: [u8; 16],
}
impl Packet for ServerboundHelloPacket {
    const ID: i32 = 0;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_utf(&self.name, MAX_LOGIN_NAME_LENGTH)?;
        output.write_bytes(&self.profile_id);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let name = input.read_utf(MAX_LOGIN_NAME_LENGTH)?;
        let profile_id = input.read_bytes(16)?;
        let profile_id = profile_id
            .try_into()
            .expect("read_bytes(16) must return exactly 16 bytes");
        Ok(Self { name, profile_id })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundLoginDisconnectPacket {
    pub reason_json: String,
}
impl Packet for ClientboundLoginDisconnectPacket {
    const ID: i32 = 0;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_utf(&self.reason_json, MAX_DISCONNECT_JSON_LENGTH)
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            reason_json: input.read_utf(MAX_DISCONNECT_JSON_LENGTH)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameProfile {
    pub id: [u8; 16],
    pub name: String,
    pub properties: Vec<GameProfileProperty>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameProfileProperty {
    pub name: String,
    pub value: String,
    pub signature: Option<String>,
}
impl GameProfile {
    pub fn offline(id: [u8; 16], name: String) -> Self {
        Self {
            id,
            name,
            properties: Vec::new(),
        }
    }
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_uuid_bytes(&self.id);
        output.write_utf(&self.name, MAX_LOGIN_NAME_LENGTH)?;
        if self.properties.len() > 16 {
            return Err(BufferError::CollectionTooLong {
                actual: self.properties.len(),
                max: 16,
            });
        }
        output.write_var_i32(self.properties.len() as i32);
        for property in &self.properties {
            output.write_utf(&property.name, 64)?;
            output.write_utf(&property.value, 32767)?;
            match &property.signature {
                Some(signature) => {
                    output.write_bool(true);
                    output.write_utf(signature, 1024)?;
                }
                None => output.write_bool(false),
            }
        }
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let id = input.read_uuid_bytes()?;
        let name = input.read_utf(MAX_LOGIN_NAME_LENGTH)?;
        let property_count = input.read_var_i32()?;
        if property_count < 0 {
            return Err(BufferError::NegativeLength);
        }
        if property_count > 16 {
            return Err(BufferError::CollectionTooLong {
                actual: property_count as usize,
                max: 16,
            });
        }
        let mut properties = Vec::with_capacity(property_count as usize);
        for _ in 0..property_count {
            let name = input.read_utf(64)?;
            let value = input.read_utf(32767)?;
            let signature = if input.read_bool()? {
                Some(input.read_utf(1024)?)
            } else {
                None
            };
            properties.push(GameProfileProperty {
                name,
                value,
                signature,
            });
        }
        Ok(Self {
            id,
            name,
            properties,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundLoginFinishedPacket {
    pub game_profile: GameProfile,
    pub session_id: [u8; 16],
}
impl Packet for ClientboundLoginFinishedPacket {
    const ID: i32 = 2;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        self.game_profile.encode(output)?;
        output.write_uuid_bytes(&self.session_id);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            game_profile: GameProfile::decode(input)?,
            session_id: input.read_uuid_bytes()?,
        })
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServerboundLoginAcknowledgedPacket;
impl Packet for ServerboundLoginAcknowledgedPacket {
    const ID: i32 = 3;
    fn encode(&self, _output: &mut PacketBuffer) -> Result<(), BufferError> {
        Ok(())
    }
    fn decode(_input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self)
    }
}
