use crate::{BufferError, Packet, PacketBuffer};
use el_nbt::Tag;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClientboundFinishConfigurationPacket;
impl Packet for ClientboundFinishConfigurationPacket {
    const ID: i32 = 3;
    fn encode(&self, _output: &mut PacketBuffer) -> Result<(), BufferError> {
        Ok(())
    }
    fn decode(_input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServerboundFinishConfigurationPacket;
impl Packet for ServerboundFinishConfigurationPacket {
    const ID: i32 = 3;
    fn encode(&self, _output: &mut PacketBuffer) -> Result<(), BufferError> {
        Ok(())
    }
    fn decode(_input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundRegistryDataPacket {
    pub registry: String,
    pub entries: Vec<PackedRegistryEntry>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PackedRegistryEntry {
    pub id: String,
    pub data: Option<Tag>,
}
impl Packet for ClientboundRegistryDataPacket {
    const ID: i32 = 7;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_utf(&self.registry, 32767)?;
        output.write_var_i32(self.entries.len() as i32);
        for entry in &self.entries {
            output.write_utf(&entry.id, 32767)?;
            output.write_nbt_tag(entry.data.as_ref())?;
        }
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let registry = input.read_utf(32767)?;
        let count = input.read_var_i32()?;
        if count < 0 {
            return Err(BufferError::NegativeLength);
        }
        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            entries.push(PackedRegistryEntry {
                id: input.read_utf(32767)?,
                data: input.read_nbt_tag()?,
            });
        }
        Ok(Self { registry, entries })
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClientboundUpdateEnabledFeaturesPacket {
    pub features: Vec<String>,
}
impl Packet for ClientboundUpdateEnabledFeaturesPacket {
    const ID: i32 = 12;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.features.len() as i32);
        for feature in &self.features {
            output.write_utf(feature, 32767)?;
        }
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let count = input.read_var_i32()?;
        if count < 0 {
            return Err(BufferError::NegativeLength);
        }
        let mut features = Vec::with_capacity(count as usize);
        for _ in 0..count {
            features.push(input.read_utf(32767)?);
        }
        Ok(Self { features })
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClientboundSelectKnownPacksPacket {
    pub known_packs: Vec<KnownPack>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}
impl Packet for ClientboundSelectKnownPacksPacket {
    const ID: i32 = 14;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        if self.known_packs.len() > 64 {
            return Err(BufferError::CollectionTooLong {
                actual: self.known_packs.len(),
                max: 64,
            });
        }
        output.write_var_i32(self.known_packs.len() as i32);
        for pack in &self.known_packs {
            output.write_utf(&pack.namespace, 32767)?;
            output.write_utf(&pack.id, 32767)?;
            output.write_utf(&pack.version, 32767)?;
        }
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let count = input.read_var_i32()?;
        if count < 0 {
            return Err(BufferError::NegativeLength);
        }
        if count > 64 {
            return Err(BufferError::CollectionTooLong {
                actual: count as usize,
                max: 64,
            });
        }
        let mut known_packs = Vec::with_capacity(count as usize);
        for _ in 0..count {
            known_packs.push(KnownPack {
                namespace: input.read_utf(32767)?,
                id: input.read_utf(32767)?,
                version: input.read_utf(32767)?,
            });
        }
        Ok(Self { known_packs })
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServerboundSelectKnownPacksPacket {
    pub known_packs: Vec<KnownPack>,
}
impl Packet for ServerboundSelectKnownPacksPacket {
    const ID: i32 = 7;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        if self.known_packs.len() > 64 {
            return Err(BufferError::CollectionTooLong {
                actual: self.known_packs.len(),
                max: 64,
            });
        }
        output.write_var_i32(self.known_packs.len() as i32);
        for pack in &self.known_packs {
            output.write_utf(&pack.namespace, 32767)?;
            output.write_utf(&pack.id, 32767)?;
            output.write_utf(&pack.version, 32767)?;
        }
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let clientbound = ClientboundSelectKnownPacksPacket::decode(input)?;
        Ok(Self {
            known_packs: clientbound.known_packs,
        })
    }
}
