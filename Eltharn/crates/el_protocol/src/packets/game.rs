use crate::{BufferError, Packet, PacketBuffer};
use el_nbt::Tag;
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundLoginPacket {
    pub player_id: i32,
    pub hardcore: bool,
    pub levels: Vec<String>,
    pub max_players: i32,
    pub chunk_radius: i32,
    pub simulation_distance: i32,
    pub reduced_debug_info: bool,
    pub show_death_screen: bool,
    pub do_limited_crafting: bool,
    pub spawn_info: CommonPlayerSpawnInfo,
    pub online_mode: bool,
    pub enforces_secure_chat: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CommonPlayerSpawnInfo {
    pub dimension_type_id: i32,
    pub dimension: String,
    pub seed: i64,
    pub game_type: GameType,
    pub previous_game_type: Option<GameType>,
    pub is_debug: bool,
    pub is_flat: bool,
    pub last_death_location: Option<GlobalPos>,
    pub portal_cooldown: i32,
    pub sea_level: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalPos {
    pub dimension: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
impl Packet for ClientboundLoginPacket {
    const ID: i32 = 49;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i32(self.player_id);
        output.write_bool(self.hardcore);
        write_identifier_collection(output, &self.levels)?;
        output.write_var_i32(self.max_players);
        output.write_var_i32(self.chunk_radius);
        output.write_var_i32(self.simulation_distance);
        output.write_bool(self.reduced_debug_info);
        output.write_bool(self.show_death_screen);
        output.write_bool(self.do_limited_crafting);
        self.spawn_info.encode(output)?;
        output.write_bool(self.online_mode);
        output.write_bool(self.enforces_secure_chat);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            player_id: input.read_i32()?,
            hardcore: input.read_bool()?,
            levels: read_identifier_collection(input)?,
            max_players: input.read_var_i32()?,
            chunk_radius: input.read_var_i32()?,
            simulation_distance: input.read_var_i32()?,
            reduced_debug_info: input.read_bool()?,
            show_death_screen: input.read_bool()?,
            do_limited_crafting: input.read_bool()?,
            spawn_info: CommonPlayerSpawnInfo::decode(input)?,
            online_mode: input.read_bool()?,
            enforces_secure_chat: input.read_bool()?,
        })
    }
}
impl CommonPlayerSpawnInfo {
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.dimension_type_id);
        output.write_utf(&self.dimension, 32767)?;
        output.write_i64(self.seed);
        output.write_i8(self.game_type.id());
        output.write_i8(self.previous_game_type.map(GameType::id).unwrap_or(-1));
        output.write_bool(self.is_debug);
        output.write_bool(self.is_flat);
        write_optional_global_pos(output, self.last_death_location.as_ref())?;
        output.write_var_i32(self.portal_cooldown);
        output.write_var_i32(self.sea_level);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            dimension_type_id: input.read_var_i32()?,
            dimension: input.read_utf(32767)?,
            seed: input.read_i64()?,
            game_type: GameType::from_id(input.read_i8()?)?,
            previous_game_type: GameType::from_nullable_id(input.read_i8()?)?,
            is_debug: input.read_bool()?,
            is_flat: input.read_bool()?,
            last_death_location: read_optional_global_pos(input)?,
            portal_cooldown: input.read_var_i32()?,
            sea_level: input.read_var_i32()?,
        })
    }
}
impl GameType {
    fn id(self) -> i8 {
        match self {
            GameType::Survival => 0,
            GameType::Creative => 1,
            GameType::Adventure => 2,
            GameType::Spectator => 3,
        }
    }
    fn from_id(id: i8) -> Result<Self, BufferError> {
        match id {
            0 => Ok(GameType::Survival),
            1 => Ok(GameType::Creative),
            2 => Ok(GameType::Adventure),
            3 => Ok(GameType::Spectator),
            _ => Err(BufferError::InvalidEnumId {
                name: "GameType",
                id: id as i32,
            }),
        }
    }
    fn from_nullable_id(id: i8) -> Result<Option<Self>, BufferError> {
        if id == -1 {
            Ok(None)
        } else {
            Self::from_id(id).map(Some)
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3d {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PositionMoveRotation {
    pub position: Vec3d,
    pub delta_movement: Vec3d,
    pub y_rot: f32,
    pub x_rot: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relative {
    X,
    Y,
    Z,
    YRot,
    XRot,
    DeltaX,
    DeltaY,
    DeltaZ,
    RotateDelta,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundPlayerPositionPacket {
    pub id: i32,
    pub change: PositionMoveRotation,
    pub relatives: Vec<Relative>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundPlayerRotationPacket {
    pub y_rot: f32,
    pub relative_y: bool,
    pub x_rot: f32,
    pub relative_x: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundLevelChunkWithLightPacket {
    pub x: i32,
    pub z: i32,
    pub chunk_data: LevelChunkPacketData,
    pub light_data: LightUpdatePacketData,
}
#[derive(Clone, Debug, PartialEq)]
pub struct HeightmapEntry {
    pub heightmap_type: i32,
    pub data: Vec<i64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LevelChunkPacketData {
    pub heightmaps: Vec<HeightmapEntry>,
    pub section_data: Vec<u8>,
    pub block_entities: Vec<BlockEntityInfo>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BlockEntityInfo {
    pub packed_xz: i8,
    pub y: i16,
    pub block_entity_type_id: i32,
    pub tag: Option<Tag>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LightUpdatePacketData {
    pub sky_y_mask: Vec<i64>,
    pub block_y_mask: Vec<i64>,
    pub empty_sky_y_mask: Vec<i64>,
    pub empty_block_y_mask: Vec<i64>,
    pub sky_updates: Vec<Vec<u8>>,
    pub block_updates: Vec<Vec<u8>>,
}
impl Packet for ClientboundPlayerPositionPacket {
    const ID: i32 = 72;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.id);
        self.change.encode(output);
        output.write_i32(pack_relatives(&self.relatives));
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            id: input.read_var_i32()?,
            change: PositionMoveRotation::decode(input)?,
            relatives: unpack_relatives(input.read_i32()?),
        })
    }
}
impl Packet for ClientboundPlayerRotationPacket {
    const ID: i32 = 73;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_f32(self.y_rot);
        output.write_bool(self.relative_y);
        output.write_f32(self.x_rot);
        output.write_bool(self.relative_x);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            y_rot: input.read_f32()?,
            relative_y: input.read_bool()?,
            x_rot: input.read_f32()?,
            relative_x: input.read_bool()?,
        })
    }
}
impl Packet for ClientboundLevelChunkWithLightPacket {
    const ID: i32 = 45;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i32(self.x);
        output.write_i32(self.z);
        self.chunk_data.encode(output)?;
        self.light_data.encode(output)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let x = input.read_i32()?;
        let z = input.read_i32()?;
        Ok(Self {
            x,
            z,
            chunk_data: LevelChunkPacketData::decode(input)?,
            light_data: LightUpdatePacketData::decode(input)?,
        })
    }
}
fn decode_heightmaps(input: &mut PacketBuffer) -> Result<Vec<HeightmapEntry>, BufferError> {
    let _heightmaps = input.read_nbt_tag()?;
    Ok(Vec::new())
}
fn encode_heightmaps(output: &mut PacketBuffer, entries: &[HeightmapEntry]) {
    output.write_var_i32(entries.len() as i32);
    for entry in entries {
        output.write_var_i32(entry.heightmap_type);
        write_long_array(output, &entry.data);
    }
}
impl LevelChunkPacketData {
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        encode_heightmaps(output, &self.heightmaps);
        if self.section_data.len() > 2_097_152 {
            return Err(BufferError::CollectionTooLong {
                actual: self.section_data.len(),
                max: 2_097_152,
            });
        }
        output.write_var_i32(self.section_data.len() as i32);
        output.write_bytes(&self.section_data);
        output.write_var_i32(self.block_entities.len() as i32);
        for block_entity in &self.block_entities {
            output.write_i8(block_entity.packed_xz);
            output.write_i16(block_entity.y);
            output.write_var_i32(block_entity.block_entity_type_id);
            output.write_nbt_tag(block_entity.tag.as_ref())?;
        }
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let heightmaps = decode_heightmaps(input)?;
        let section_data_len = input.read_var_i32()?;
        if section_data_len < 0 {
            return Err(BufferError::NegativeLength);
        }
        if section_data_len > 2_097_152 {
            return Err(BufferError::CollectionTooLong {
                actual: section_data_len as usize,
                max: 2_097_152,
            });
        }
        let section_data = input.read_bytes(section_data_len as usize)?;
        let block_entity_count = input.read_var_i32()?;
        if block_entity_count < 0 {
            return Err(BufferError::NegativeLength);
        }
        let mut block_entities = Vec::with_capacity(block_entity_count as usize);
        for _ in 0..block_entity_count {
            block_entities.push(BlockEntityInfo {
                packed_xz: input.read_i8()?,
                y: input.read_i16()?,
                block_entity_type_id: input.read_var_i32()?,
                tag: input.read_nbt_tag()?,
            });
        }
        Ok(Self {
            heightmaps,
            section_data,
            block_entities,
        })
    }
}
impl LightUpdatePacketData {
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        write_long_array(output, &self.sky_y_mask);
        write_long_array(output, &self.block_y_mask);
        write_long_array(output, &self.empty_sky_y_mask);
        write_long_array(output, &self.empty_block_y_mask);
        write_byte_array_list(output, &self.sky_updates)?;
        write_byte_array_list(output, &self.block_updates)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            sky_y_mask: read_long_array(input)?,
            block_y_mask: read_long_array(input)?,
            empty_sky_y_mask: read_long_array(input)?,
            empty_block_y_mask: read_long_array(input)?,
            sky_updates: read_byte_array_list(input, 2048)?,
            block_updates: read_byte_array_list(input, 2048)?,
        })
    }
}
impl PositionMoveRotation {
    fn encode(self, output: &mut PacketBuffer) {
        self.position.encode(output);
        self.delta_movement.encode(output);
        output.write_f32(self.y_rot);
        output.write_f32(self.x_rot);
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            position: Vec3d::decode(input)?,
            delta_movement: Vec3d::decode(input)?,
            y_rot: input.read_f32()?,
            x_rot: input.read_f32()?,
        })
    }
}
impl Vec3d {
    fn encode(self, output: &mut PacketBuffer) {
        output.write_f64(self.x);
        output.write_f64(self.y);
        output.write_f64(self.z);
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            x: input.read_f64()?,
            y: input.read_f64()?,
            z: input.read_f64()?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientboundSetChunkCacheCenterPacket {
    pub x: i32,
    pub z: i32,
}
impl Packet for ClientboundSetChunkCacheCenterPacket {
    const ID: i32 = 94;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.x);
        output.write_var_i32(self.z);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            x: input.read_var_i32()?,
            z: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientboundSetChunkCacheRadiusPacket {
    pub radius: i32,
}
impl Packet for ClientboundSetChunkCacheRadiusPacket {
    const ID: i32 = 95;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.radius);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            radius: input.read_var_i32()?,
        })
    }
}
fn write_identifier_collection(
    output: &mut PacketBuffer,
    values: &[String],
) -> Result<(), BufferError> {
    output.write_var_i32(values.len() as i32);
    for value in values {
        output.write_utf(value, 32767)?;
    }
    Ok(())
}
fn read_identifier_collection(input: &mut PacketBuffer) -> Result<Vec<String>, BufferError> {
    let count = input.read_var_i32()?;
    if count < 0 {
        return Err(BufferError::NegativeLength);
    }
    let mut values = Vec::with_capacity(count as usize);
    for _ in 0..count {
        values.push(input.read_utf(32767)?);
    }
    Ok(values)
}
fn write_optional_global_pos(
    output: &mut PacketBuffer,
    value: Option<&GlobalPos>,
) -> Result<(), BufferError> {
    match value {
        Some(value) => {
            output.write_bool(true);
            output.write_utf(&value.dimension, 32767)?;
            let val = (((value.x as i64) & 0x3FFFFFF) << 38)
                | (((value.z as i64) & 0x3FFFFFF) << 12)
                | ((value.y as i64) & 0xFFF);
            output.write_i64(val);
        }
        None => output.write_bool(false),
    }
    Ok(())
}
fn read_optional_global_pos(input: &mut PacketBuffer) -> Result<Option<GlobalPos>, BufferError> {
    if !input.read_bool()? {
        return Ok(None);
    }
    let dimension = input.read_utf(32767)?;
    let val = input.read_i64()?;
    let mut x = val >> 38;
    let mut z = (val >> 12) & 0x3FFFFFF;
    let mut y = val & 0xFFF;
    if x >= 1 << 25 {
        x -= 1 << 26;
    }
    if z >= 1 << 25 {
        z -= 1 << 26;
    }
    if y >= 1 << 11 {
        y -= 1 << 12;
    }
    Ok(Some(GlobalPos {
        dimension,
        x: x as i32,
        y: y as i32,
        z: z as i32,
    }))
}
fn write_long_array(output: &mut PacketBuffer, values: &[i64]) {
    output.write_var_i32(values.len() as i32);
    for value in values {
        output.write_i64(*value);
    }
}
fn read_long_array(input: &mut PacketBuffer) -> Result<Vec<i64>, BufferError> {
    let count = input.read_var_i32()?;
    if count < 0 {
        return Err(BufferError::NegativeLength);
    }
    let mut values = Vec::with_capacity(count as usize);
    for _ in 0..count {
        values.push(input.read_i64()?);
    }
    Ok(values)
}
fn write_byte_array_list(output: &mut PacketBuffer, values: &[Vec<u8>]) -> Result<(), BufferError> {
    output.write_var_i32(values.len() as i32);
    for value in values {
        if value.len() > 2048 {
            return Err(BufferError::CollectionTooLong {
                actual: value.len(),
                max: 2048,
            });
        }
        output.write_var_i32(value.len() as i32);
        output.write_bytes(value);
    }
    Ok(())
}
fn read_byte_array_list(
    input: &mut PacketBuffer,
    max_len: usize,
) -> Result<Vec<Vec<u8>>, BufferError> {
    let count = input.read_var_i32()?;
    if count < 0 {
        return Err(BufferError::NegativeLength);
    }
    let mut values = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let len = input.read_var_i32()?;
        if len < 0 {
            return Err(BufferError::NegativeLength);
        }
        if len as usize > max_len {
            return Err(BufferError::CollectionTooLong {
                actual: len as usize,
                max: max_len,
            });
        }
        values.push(input.read_bytes(len as usize)?);
    }
    Ok(values)
}
fn pack_relatives(values: &[Relative]) -> i32 {
    values
        .iter()
        .fold(0, |packed, relative| packed | relative.mask())
}
fn unpack_relatives(value: i32) -> Vec<Relative> {
    Relative::VALUES
        .iter()
        .copied()
        .filter(|relative| value & relative.mask() == relative.mask())
        .collect()
}
impl Relative {
    const VALUES: [Relative; 9] = [
        Relative::X,
        Relative::Y,
        Relative::Z,
        Relative::YRot,
        Relative::XRot,
        Relative::DeltaX,
        Relative::DeltaY,
        Relative::DeltaZ,
        Relative::RotateDelta,
    ];
    fn mask(self) -> i32 {
        1 << match self {
            Relative::X => 0,
            Relative::Y => 1,
            Relative::Z => 2,
            Relative::YRot => 3,
            Relative::XRot => 4,
            Relative::DeltaX => 5,
            Relative::DeltaY => 6,
            Relative::DeltaZ => 7,
            Relative::RotateDelta => 8,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundAcceptTeleportationPacket {
    pub id: i32,
}
impl Packet for ServerboundAcceptTeleportationPacket {
    const ID: i32 = 0x00;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.id);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            id: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundAttackPacket {
    pub entity_id: i32,
}
impl Packet for ServerboundAttackPacket {
    const ID: i32 = 0x01;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.entity_id);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            entity_id: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundPlayerLoadedPacket;
impl Packet for ServerboundPlayerLoadedPacket {
    const ID: i32 = 44;
    fn encode(&self, _output: &mut PacketBuffer) -> Result<(), BufferError> {
        Ok(())
    }
    fn decode(_input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundClientTickEndPacket;
impl Packet for ServerboundClientTickEndPacket {
    const ID: i32 = 13;
    fn encode(&self, _output: &mut PacketBuffer) -> Result<(), BufferError> {
        Ok(())
    }
    fn decode(_input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundChunkBatchStartPacket;
impl Packet for ClientboundChunkBatchStartPacket {
    const ID: i32 = 12;
    fn encode(&self, _output: &mut PacketBuffer) -> Result<(), BufferError> {
        Ok(())
    }
    fn decode(_input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundChunkBatchFinishedPacket {
    pub batch_size: i32,
}
impl Packet for ClientboundChunkBatchFinishedPacket {
    const ID: i32 = 11;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.batch_size);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            batch_size: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundChunkBatchReceivedPacket {
    pub desired_chunks_per_tick: f32,
}
impl Packet for ServerboundChunkBatchReceivedPacket {
    const ID: i32 = 11;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_f32(self.desired_chunks_per_tick);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            desired_chunks_per_tick: input.read_f32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundMovePlayerPosRotPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
    pub horizontal_collision: bool,
}
impl Packet for ServerboundMovePlayerPosRotPacket {
    const ID: i32 = 31;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_f64(self.x);
        output.write_f64(self.y);
        output.write_f64(self.z);
        output.write_f32(self.y_rot);
        output.write_f32(self.x_rot);
        let mut flags = 0u8;
        if self.on_ground {
            flags |= 1;
        }
        if self.horizontal_collision {
            flags |= 2;
        }
        output.write_u8(flags);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let x = input.read_f64()?;
        let y = input.read_f64()?;
        let z = input.read_f64()?;
        let y_rot = input.read_f32()?;
        let x_rot = input.read_f32()?;
        let flags = input.read_u8()?;
        Ok(Self {
            x,
            y,
            z,
            y_rot,
            x_rot,
            on_ground: (flags & 1) != 0,
            horizontal_collision: (flags & 2) != 0,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundMovePlayerPosPacket {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub on_ground: bool,
    pub horizontal_collision: bool,
}
impl Packet for ServerboundMovePlayerPosPacket {
    const ID: i32 = 30;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_f64(self.x);
        output.write_f64(self.y);
        output.write_f64(self.z);
        let mut flags = 0u8;
        if self.on_ground {
            flags |= 1;
        }
        if self.horizontal_collision {
            flags |= 2;
        }
        output.write_u8(flags);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let x = input.read_f64()?;
        let y = input.read_f64()?;
        let z = input.read_f64()?;
        let flags = input.read_u8()?;
        Ok(Self {
            x,
            y,
            z,
            on_ground: (flags & 1) != 0,
            horizontal_collision: (flags & 2) != 0,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundMovePlayerRotPacket {
    pub y_rot: f32,
    pub x_rot: f32,
    pub on_ground: bool,
    pub horizontal_collision: bool,
}
impl Packet for ServerboundMovePlayerRotPacket {
    const ID: i32 = 32;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_f32(self.y_rot);
        output.write_f32(self.x_rot);
        let mut flags = 0u8;
        if self.on_ground {
            flags |= 1;
        }
        if self.horizontal_collision {
            flags |= 2;
        }
        output.write_u8(flags);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let y_rot = input.read_f32()?;
        let x_rot = input.read_f32()?;
        let flags = input.read_u8()?;
        Ok(Self {
            y_rot,
            x_rot,
            on_ground: (flags & 1) != 0,
            horizontal_collision: (flags & 2) != 0,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundMovePlayerStatusOnlyPacket {
    pub on_ground: bool,
    pub horizontal_collision: bool,
}
impl Packet for ServerboundMovePlayerStatusOnlyPacket {
    const ID: i32 = 33;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        let mut flags = 0u8;
        if self.on_ground {
            flags |= 1;
        }
        if self.horizontal_collision {
            flags |= 2;
        }
        output.write_u8(flags);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let flags = input.read_u8()?;
        Ok(Self {
            on_ground: (flags & 1) != 0,
            horizontal_collision: (flags & 2) != 0,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtoBlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
fn write_proto_block_pos(output: &mut PacketBuffer, pos: &ProtoBlockPos) {
    let val = (((pos.x as i64) & 0x3FFFFFF) << 38)
        | (((pos.z as i64) & 0x3FFFFFF) << 12)
        | ((pos.y as i64) & 0xFFF);
    output.write_i64(val);
}
fn read_proto_block_pos(input: &mut PacketBuffer) -> Result<ProtoBlockPos, BufferError> {
    let val = input.read_i64()?;
    let mut x = val >> 38;
    let mut z = (val >> 12) & 0x3FFFFFF;
    let mut y = val & 0xFFF;
    if x >= 1 << 25 {
        x -= 1 << 26;
    }
    if z >= 1 << 25 {
        z -= 1 << 26;
    }
    if y >= 1 << 11 {
        y -= 1 << 12;
    }
    Ok(ProtoBlockPos {
        x: x as i32,
        y: y as i32,
        z: z as i32,
    })
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundPlayerActionPacket {
    pub action: i32,
    pub pos: ProtoBlockPos,
    pub direction: i8,
    pub sequence: i32,
}
impl Packet for ServerboundPlayerActionPacket {
    const ID: i32 = 41;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.action);
        write_proto_block_pos(output, &self.pos);
        output.write_i8(self.direction);
        output.write_var_i32(self.sequence);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            action: input.read_var_i32()?,
            pos: read_proto_block_pos(input)?,
            direction: input.read_i8()?,
            sequence: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServerboundPlayerInputPacket {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub jump: bool,
    pub shift: bool,
    pub sprint: bool,
}
impl Packet for ServerboundPlayerInputPacket {
    const ID: i32 = 43;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        let mut flags = 0u8;
        flags |= u8::from(self.forward);
        flags |= u8::from(self.backward) << 1;
        flags |= u8::from(self.left) << 2;
        flags |= u8::from(self.right) << 3;
        flags |= u8::from(self.jump) << 4;
        flags |= u8::from(self.shift) << 5;
        flags |= u8::from(self.sprint) << 6;
        output.write_u8(flags);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let flags = input.read_u8()?;
        Ok(Self {
            forward: flags & 1 != 0,
            backward: flags & 2 != 0,
            left: flags & 4 != 0,
            right: flags & 8 != 0,
            jump: flags & 16 != 0,
            shift: flags & 32 != 0,
            sprint: flags & 64 != 0,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundChangeGameModePacket {
    pub game_mode: i32,
}
impl Packet for ServerboundChangeGameModePacket {
    const ID: i32 = 5;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.game_mode);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            game_mode: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundPlayerAbilitiesPacket {
    pub is_flying: bool,
}
impl Packet for ServerboundPlayerAbilitiesPacket {
    const ID: i32 = 40;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        let bitfield = if self.is_flying { 2i8 } else { 0i8 };
        output.write_i8(bitfield);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let bitfield = input.read_i8()?;
        let is_flying = (bitfield & 2) != 0;
        Ok(Self { is_flying })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundUseItemOnPacket {
    pub hand: i32,
    pub block_pos: ProtoBlockPos,
    pub direction: i32,
    pub cursor_x: f32,
    pub cursor_y: f32,
    pub cursor_z: f32,
    pub inside_block: bool,
    pub world_border: bool,
    pub sequence: i32,
}
impl Packet for ServerboundUseItemOnPacket {
    const ID: i32 = 66;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.hand);
        write_proto_block_pos(output, &self.block_pos);
        output.write_var_i32(self.direction);
        output.write_f32(self.cursor_x);
        output.write_f32(self.cursor_y);
        output.write_f32(self.cursor_z);
        output.write_bool(self.inside_block);
        output.write_bool(self.world_border);
        output.write_var_i32(self.sequence);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            hand: input.read_var_i32()?,
            block_pos: read_proto_block_pos(input)?,
            direction: input.read_var_i32()?,
            cursor_x: input.read_f32()?,
            cursor_y: input.read_f32()?,
            cursor_z: input.read_f32()?,
            inside_block: input.read_bool()?,
            world_border: input.read_bool()?,
            sequence: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundCommandSuggestionPacket {
    pub id: i32,
    pub command: String,
}
impl Packet for ServerboundCommandSuggestionPacket {
    const ID: i32 = 15;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.id);
        output.write_utf(&self.command, 32500)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            id: input.read_var_i32()?,
            command: input.read_utf(32500)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundCommandSuggestionsPacket {
    pub id: i32,
    pub start: usize,
    pub length: usize,
    pub suggestions: Vec<CommandSuggestionEntry>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSuggestionEntry {
    pub text: String,
    pub tooltip: Option<String>,
}
impl Packet for ClientboundCommandSuggestionsPacket {
    const ID: i32 = 15;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.id);
        output.write_var_i32(self.start as i32);
        output.write_var_i32(self.length as i32);
        output.write_var_i32(self.suggestions.len() as i32);
        for suggestion in &self.suggestions {
            suggestion.encode(output)?;
        }
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let id = input.read_var_i32()?;
        let start = read_non_negative_var_usize(input)?;
        let length = read_non_negative_var_usize(input)?;
        let count = input.read_var_i32()?;
        if count < 0 {
            return Err(BufferError::NegativeLength);
        }
        let mut suggestions = Vec::with_capacity(count as usize);
        for _ in 0..count {
            suggestions.push(CommandSuggestionEntry::decode(input)?);
        }
        Ok(Self {
            id,
            start,
            length,
            suggestions,
        })
    }
}
impl CommandSuggestionEntry {
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_utf(&self.text, 32767)?;
        write_optional_component_string(output, self.tooltip.as_ref())?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            text: input.read_utf(32767)?,
            tooltip: read_optional_component_string(input)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundChatCommandPacket {
    pub command: String,
}
impl Packet for ServerboundChatCommandPacket {
    const ID: i32 = 7;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_utf(&self.command, 32767)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            command: input.read_utf(32767)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundSystemChatPacket {
    pub content: String,
    pub overlay: bool,
}
impl Packet for ClientboundSystemChatPacket {
    const ID: i32 = 121;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        let tag = Tag::String(self.content.clone());
        output.write_nbt_tag(Some(&tag))?;
        output.write_bool(self.overlay);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let tag = input
            .read_nbt_tag()?
            .ok_or_else(|| BufferError::Nbt("Expected non-empty tag".to_string()))?;
        let content = match tag {
            Tag::String(s) => s,
            _ => "{}".to_string(),
        };
        let overlay = input.read_bool()?;
        Ok(Self { content, overlay })
    }
}
fn read_non_negative_var_usize(input: &mut PacketBuffer) -> Result<usize, BufferError> {
    let value = input.read_var_i32()?;
    if value < 0 {
        Err(BufferError::NegativeLength)
    } else {
        Ok(value as usize)
    }
}
fn write_optional_component_string(
    output: &mut PacketBuffer,
    value: Option<&String>,
) -> Result<(), BufferError> {
    output.write_bool(value.is_some());
    if let Some(value) = value {
        output.write_nbt_tag(Some(&Tag::String(value.clone())))?;
    }
    Ok(())
}
fn read_optional_component_string(input: &mut PacketBuffer) -> Result<Option<String>, BufferError> {
    if !input.read_bool()? {
        return Ok(None);
    }
    let tag = input
        .read_nbt_tag()?
        .ok_or_else(|| BufferError::Nbt("Expected optional component tag".to_string()))?;
    Ok(Some(component_tag_to_plain_text(tag)))
}
fn component_tag_to_plain_text(tag: Tag) -> String {
    match tag {
        Tag::String(value) => value,
        Tag::Compound(fields) => fields
            .iter()
            .find_map(|(key, value)| match (key.as_str(), value) {
                ("text", Tag::String(text)) => Some(text.clone()),
                _ => None,
            })
            .unwrap_or_else(|| format!("{fields:?}")),
        other => format!("{other:?}"),
    }
}
fn compound_get<'a>(fields: &'a [(String, Tag)], key: &str) -> Option<&'a Tag> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
fn get_potion_id(name: &str) -> Option<i32> {
    let path = name.strip_prefix("minecraft:").unwrap_or(name);
    match path {
        "water" => Some(0),
        "mundane" => Some(1),
        "thick" => Some(2),
        "awkward" => Some(3),
        "night_vision" => Some(4),
        "long_night_vision" => Some(5),
        "invisibility" => Some(6),
        "long_invisibility" => Some(7),
        "leaping" => Some(8),
        "long_leaping" => Some(9),
        "strong_leaping" => Some(10),
        "fire_resistance" => Some(11),
        "long_fire_resistance" => Some(12),
        "swiftness" => Some(13),
        "long_swiftness" => Some(14),
        "strong_swiftness" => Some(15),
        "slowness" => Some(16),
        "long_slowness" => Some(17),
        "strong_slowness" => Some(18),
        "turtle_master" => Some(19),
        "long_turtle_master" => Some(20),
        "strong_turtle_master" => Some(21),
        "water_breathing" => Some(22),
        "long_water_breathing" => Some(23),
        "healing" => Some(24),
        "strong_healing" => Some(25),
        "harming" => Some(26),
        "strong_harming" => Some(27),
        "poison" => Some(28),
        "long_poison" => Some(29),
        "strong_poison" => Some(30),
        "regeneration" => Some(31),
        "long_regeneration" => Some(32),
        "strong_regeneration" => Some(33),
        "strength" => Some(34),
        "long_strength" => Some(35),
        "strong_strength" => Some(36),
        "weakness" => Some(37),
        "long_weakness" => Some(38),
        "luck" => Some(39),
        "slow_falling" => Some(40),
        "long_slow_falling" => Some(41),
        "wind_charged" => Some(42),
        "weaving" => Some(43),
        "oozing" => Some(44),
        "infested" => Some(45),
        _ => None,
    }
}
fn get_potion_name_by_id(id: i32) -> Option<&'static str> {
    match id {
        0 => Some("minecraft:water"),
        1 => Some("minecraft:mundane"),
        2 => Some("minecraft:thick"),
        3 => Some("minecraft:awkward"),
        4 => Some("minecraft:night_vision"),
        5 => Some("minecraft:long_night_vision"),
        6 => Some("minecraft:invisibility"),
        7 => Some("minecraft:long_invisibility"),
        8 => Some("minecraft:leaping"),
        9 => Some("minecraft:long_leaping"),
        10 => Some("minecraft:strong_leaping"),
        11 => Some("minecraft:fire_resistance"),
        12 => Some("minecraft:long_fire_resistance"),
        13 => Some("minecraft:swiftness"),
        14 => Some("minecraft:long_swiftness"),
        15 => Some("minecraft:strong_swiftness"),
        16 => Some("minecraft:slowness"),
        17 => Some("minecraft:long_slowness"),
        18 => Some("minecraft:strong_slowness"),
        19 => Some("minecraft:turtle_master"),
        20 => Some("minecraft:long_turtle_master"),
        21 => Some("minecraft:strong_turtle_master"),
        22 => Some("minecraft:water_breathing"),
        23 => Some("minecraft:long_water_breathing"),
        24 => Some("minecraft:healing"),
        25 => Some("minecraft:strong_healing"),
        26 => Some("minecraft:harming"),
        27 => Some("minecraft:strong_harming"),
        28 => Some("minecraft:poison"),
        29 => Some("minecraft:long_poison"),
        30 => Some("minecraft:strong_poison"),
        31 => Some("minecraft:regeneration"),
        32 => Some("minecraft:long_regeneration"),
        33 => Some("minecraft:strong_regeneration"),
        34 => Some("minecraft:strength"),
        35 => Some("minecraft:long_strength"),
        36 => Some("minecraft:strong_strength"),
        37 => Some("minecraft:weakness"),
        38 => Some("minecraft:long_weakness"),
        39 => Some("minecraft:luck"),
        40 => Some("minecraft:slow_falling"),
        41 => Some("minecraft:long_slow_falling"),
        42 => Some("minecraft:wind_charged"),
        43 => Some("minecraft:weaving"),
        44 => Some("minecraft:oozing"),
        45 => Some("minecraft:infested"),
        _ => None,
    }
}
fn get_enchantment_id(name: &str) -> Option<i32> {
    let path = name.strip_prefix("minecraft:").unwrap_or(name);
    match path {
        "channeling" => Some(0),
        "quick_charge" => Some(1),
        "silk_touch" => Some(2),
        "protection" => Some(3),
        "feather_falling" => Some(4),
        "sweeping_edge" => Some(5),
        "impaling" => Some(6),
        "wind_burst" => Some(7),
        "sharpness" => Some(8),
        "power" => Some(9),
        "lure" => Some(10),
        "unbreaking" => Some(11),
        "depth_strider" => Some(12),
        "loyalty" => Some(13),
        "smite" => Some(14),
        "fire_aspect" => Some(15),
        "breach" => Some(16),
        "mending" => Some(17),
        "binding_curse" => Some(18),
        "frost_walker" => Some(19),
        "soul_speed" => Some(20),
        "piercing" => Some(21),
        "vanishing_curse" => Some(22),
        "looting" => Some(23),
        "projectile_protection" => Some(24),
        "blast_protection" => Some(25),
        "respiration" => Some(26),
        "multishot" => Some(27),
        "thorns" => Some(28),
        "riptide" => Some(29),
        "flame" => Some(30),
        "infinity" => Some(31),
        "aqua_affinity" => Some(32),
        "efficiency" => Some(33),
        "knockback" => Some(34),
        "fire_protection" => Some(35),
        "fortune" => Some(36),
        "punch" => Some(37),
        "swift_sneak" => Some(38),
        "bane_of_arthropods" => Some(39),
        "density" => Some(40),
        "lunge" => Some(41),
        "luck_of_the_sea" => Some(42),
        _ => None,
    }
}
fn get_enchantment_name_by_id(id: i32) -> Option<&'static str> {
    match id {
        0 => Some("minecraft:channeling"),
        1 => Some("minecraft:quick_charge"),
        2 => Some("minecraft:silk_touch"),
        3 => Some("minecraft:protection"),
        4 => Some("minecraft:feather_falling"),
        5 => Some("minecraft:sweeping_edge"),
        6 => Some("minecraft:impaling"),
        7 => Some("minecraft:wind_burst"),
        8 => Some("minecraft:sharpness"),
        9 => Some("minecraft:power"),
        10 => Some("minecraft:lure"),
        11 => Some("minecraft:unbreaking"),
        12 => Some("minecraft:depth_strider"),
        13 => Some("minecraft:loyalty"),
        14 => Some("minecraft:smite"),
        15 => Some("minecraft:fire_aspect"),
        16 => Some("minecraft:breach"),
        17 => Some("minecraft:mending"),
        18 => Some("minecraft:binding_curse"),
        19 => Some("minecraft:frost_walker"),
        20 => Some("minecraft:soul_speed"),
        21 => Some("minecraft:piercing"),
        22 => Some("minecraft:vanishing_curse"),
        23 => Some("minecraft:looting"),
        24 => Some("minecraft:projectile_protection"),
        25 => Some("minecraft:blast_protection"),
        26 => Some("minecraft:respiration"),
        27 => Some("minecraft:multishot"),
        28 => Some("minecraft:thorns"),
        29 => Some("minecraft:riptide"),
        30 => Some("minecraft:flame"),
        31 => Some("minecraft:infinity"),
        32 => Some("minecraft:aqua_affinity"),
        33 => Some("minecraft:efficiency"),
        34 => Some("minecraft:knockback"),
        35 => Some("minecraft:fire_protection"),
        36 => Some("minecraft:fortune"),
        37 => Some("minecraft:punch"),
        38 => Some("minecraft:swift_sneak"),
        39 => Some("minecraft:bane_of_arthropods"),
        40 => Some("minecraft:density"),
        41 => Some("minecraft:lunge"),
        42 => Some("minecraft:luck_of_the_sea"),
        _ => None,
    }
}
fn get_mob_effect_id(name: &str) -> Option<i32> {
    let path = name.strip_prefix("minecraft:").unwrap_or(name);
    match path {
        "speed" => Some(0),
        "slowness" => Some(1),
        "haste" => Some(2),
        "mining_fatigue" => Some(3),
        "strength" => Some(4),
        "instant_health" => Some(5),
        "instant_damage" => Some(6),
        "jump_boost" => Some(7),
        "nausea" => Some(8),
        "regeneration" => Some(9),
        "resistance" => Some(10),
        "fire_resistance" => Some(11),
        "water_breathing" => Some(12),
        "invisibility" => Some(13),
        "blindness" => Some(14),
        "night_vision" => Some(15),
        "hunger" => Some(16),
        "weakness" => Some(17),
        "poison" => Some(18),
        "wither" => Some(19),
        "health_boost" => Some(20),
        "absorption" => Some(21),
        "saturation" => Some(22),
        "glowing" => Some(23),
        "levitation" => Some(24),
        "luck" => Some(25),
        "unluck" => Some(26),
        "slow_falling" => Some(27),
        "conduit_power" => Some(28),
        "dolphins_grace" => Some(29),
        "bad_omen" => Some(30),
        "hero_of_the_village" => Some(31),
        "darkness" => Some(32),
        "trial_omen" => Some(33),
        "raid_omen" => Some(34),
        "wind_charged" => Some(35),
        "weaving" => Some(36),
        "oozing" => Some(37),
        "infested" => Some(38),
        "breath_of_the_nautilus" => Some(39),
        _ => None,
    }
}
fn get_mob_effect_name_by_id(id: i32) -> Option<&'static str> {
    match id {
        0 => Some("minecraft:speed"),
        1 => Some("minecraft:slowness"),
        2 => Some("minecraft:haste"),
        3 => Some("minecraft:mining_fatigue"),
        4 => Some("minecraft:strength"),
        5 => Some("minecraft:instant_health"),
        6 => Some("minecraft:instant_damage"),
        7 => Some("minecraft:jump_boost"),
        8 => Some("minecraft:nausea"),
        9 => Some("minecraft:regeneration"),
        10 => Some("minecraft:resistance"),
        11 => Some("minecraft:fire_resistance"),
        12 => Some("minecraft:water_breathing"),
        13 => Some("minecraft:invisibility"),
        14 => Some("minecraft:blindness"),
        15 => Some("minecraft:night_vision"),
        16 => Some("minecraft:hunger"),
        17 => Some("minecraft:weakness"),
        18 => Some("minecraft:poison"),
        19 => Some("minecraft:wither"),
        20 => Some("minecraft:health_boost"),
        21 => Some("minecraft:absorption"),
        22 => Some("minecraft:saturation"),
        23 => Some("minecraft:glowing"),
        24 => Some("minecraft:levitation"),
        25 => Some("minecraft:luck"),
        26 => Some("minecraft:unluck"),
        27 => Some("minecraft:slow_falling"),
        28 => Some("minecraft:conduit_power"),
        29 => Some("minecraft:dolphins_grace"),
        30 => Some("minecraft:bad_omen"),
        31 => Some("minecraft:hero_of_the_village"),
        32 => Some("minecraft:darkness"),
        33 => Some("minecraft:trial_omen"),
        34 => Some("minecraft:raid_omen"),
        35 => Some("minecraft:wind_charged"),
        36 => Some("minecraft:weaving"),
        37 => Some("minecraft:oozing"),
        38 => Some("minecraft:infested"),
        39 => Some("minecraft:breath_of_the_nautilus"),
        _ => None,
    }
}
fn get_instrument_id(name: &str) -> Option<i32> {
    let path = name.strip_prefix("minecraft:").unwrap_or(name);
    match path {
        "admire_goat_horn" => Some(0),
        "dream_goat_horn" => Some(1),
        "call_goat_horn" => Some(2),
        "seek_goat_horn" => Some(3),
        "feel_goat_horn" => Some(4),
        "ponder_goat_horn" => Some(5),
        "yearn_goat_horn" => Some(6),
        "sing_goat_horn" => Some(7),
        _ => None,
    }
}
fn get_instrument_name_by_id(id: i32) -> Option<&'static str> {
    match id {
        0 => Some("minecraft:admire_goat_horn"),
        1 => Some("minecraft:dream_goat_horn"),
        2 => Some("minecraft:call_goat_horn"),
        3 => Some("minecraft:seek_goat_horn"),
        4 => Some("minecraft:feel_goat_horn"),
        5 => Some("minecraft:ponder_goat_horn"),
        6 => Some("minecraft:yearn_goat_horn"),
        7 => Some("minecraft:sing_goat_horn"),
        _ => None,
    }
}
fn write_item_stack(
    output: &mut PacketBuffer,
    stack: &el_core::ItemStack,
) -> Result<(), BufferError> {
    if stack.is_empty() {
        output.write_var_i32(0);
    } else {
        output.write_var_i32(stack.count);
        let item_id =
            el_core::registry::BuiltInRegistries::items().get_id(&stack.item.location) as i32;
        output.write_var_i32(item_id);
        let mut add_buf = PacketBuffer::new();
        let mut add_count = 0;
        if let Some(el_core::item::DataComponent::Damage(d)) =
            stack.components.get("minecraft:damage")
        {
            add_buf.write_var_i32(3);
            add_buf.write_var_i32(*d);
            add_count += 1;
        }
        if let Some(el_core::item::DataComponent::Generic {
            value: Tag::Compound(fields),
            ..
        }) = stack.components.get("minecraft:potion_contents")
        {
            add_buf.write_var_i32(51);
            if let Some(Tag::String(potion_name)) = compound_get(fields, "potion") {
                if let Some(potion_id) = get_potion_id(potion_name) {
                    add_buf.write_bool(true);
                    add_buf.write_var_i32(potion_id + 1);
                } else {
                    add_buf.write_bool(false);
                }
            } else {
                add_buf.write_bool(false);
            }
            if let Some(color_tag) = compound_get(fields, "custom_color") {
                let color_val = match color_tag {
                    Tag::Int(c) => *c,
                    Tag::Byte(c) => *c as i32,
                    _ => 0,
                };
                add_buf.write_bool(true);
                add_buf.write_i32(color_val);
            } else {
                add_buf.write_bool(false);
            }
            add_buf.write_var_i32(0);
            if let Some(Tag::String(name)) = compound_get(fields, "custom_name") {
                add_buf.write_bool(true);
                add_buf.write_utf(name, 32767)?;
            } else {
                add_buf.write_bool(false);
            }
            add_count += 1;
        }
        if let Some(el_core::item::DataComponent::Generic { value, .. }) =
            stack.components.get("minecraft:ominous_bottle_amplifier")
        {
            let amp = match value {
                Tag::Int(v) => *v,
                Tag::Byte(v) => *v as i32,
                Tag::Short(v) => *v as i32,
                _ => 0,
            };
            add_buf.write_var_i32(63);
            add_buf.write_var_i32(amp);
            add_count += 1;
        }
        if let Some(el_core::item::DataComponent::Generic {
            value: Tag::Compound(fields),
            ..
        }) = stack.components.get("minecraft:stored_enchantments")
        {
            add_buf.write_var_i32(42);
            add_buf.write_var_i32(fields.len() as i32);
            for (ench_name, lvl_tag) in fields {
                let level = match lvl_tag {
                    Tag::Int(l) => *l,
                    Tag::Byte(l) => *l as i32,
                    Tag::Short(l) => *l as i32,
                    _ => 1,
                };
                let ench_id = get_enchantment_id(ench_name).unwrap_or(0);
                add_buf.write_var_i32(ench_id + 1);
                add_buf.write_var_i32(level);
            }
            add_count += 1;
        }
        if let Some(el_core::item::DataComponent::Generic {
            value: Tag::String(inst_name),
            ..
        }) = stack.components.get("minecraft:instrument")
        {
            add_buf.write_var_i32(61);
            let inst_id = get_instrument_id(inst_name).unwrap_or(0);
            add_buf.write_var_i32(inst_id + 1);
            add_count += 1;
        }
        if let Some(el_core::item::DataComponent::Generic {
            value: Tag::List(effects),
            ..
        }) = stack.components.get("minecraft:suspicious_stew_effects")
        {
            add_buf.write_var_i32(53);
            add_buf.write_var_i32(effects.len() as i32);
            for tag in effects {
                if let Tag::Compound(m) = tag {
                    let effect_name = match compound_get(m, "id") {
                        Some(Tag::String(s)) => s.as_str(),
                        _ => "",
                    };
                    let effect_id = get_mob_effect_id(effect_name).unwrap_or(0);
                    let duration = match compound_get(m, "duration") {
                        Some(Tag::Int(d)) => *d,
                        Some(Tag::Byte(d)) => *d as i32,
                        Some(Tag::Short(d)) => *d as i32,
                        _ => 160,
                    };
                    add_buf.write_var_i32(effect_id + 1);
                    add_buf.write_var_i32(duration);
                } else {
                    add_buf.write_var_i32(1);
                    add_buf.write_var_i32(160);
                }
            }
            add_count += 1;
        }
        output.write_var_i32(add_count);
        output.write_bytes(&add_buf.into_inner());
        output.write_var_i32(0);
    }
    Ok(())
}
fn read_item_stack(input: &mut PacketBuffer) -> Result<el_core::ItemStack, BufferError> {
    let count = input.read_var_i32()?;
    if count <= 0 {
        Ok(el_core::ItemStack::empty())
    } else {
        let item_id = input.read_var_i32()?;
        let item = el_core::registry::BuiltInRegistries::items()
            .get_by_id(item_id as usize)
            .clone();
        let add_count = input.read_var_i32()?;
        let mut components = el_core::item::PatchedDataComponentMap::new(item.components.clone());
        for _ in 0..add_count {
            let comp_type = input.read_var_i32()?;
            match comp_type {
                3 => {
                    let d = input.read_var_i32()?;
                    components.set("minecraft:damage", el_core::item::DataComponent::Damage(d));
                }
                51 => {
                    let mut fields = Vec::new();
                    let has_potion = input.read_bool()?;
                    if has_potion {
                        let potion_id = input.read_var_i32()?;
                        if let Some(potion_name) = get_potion_name_by_id(potion_id - 1) {
                            fields
                                .push(("potion".to_string(), Tag::String(potion_name.to_string())));
                        }
                    }
                    let has_color = input.read_bool()?;
                    if has_color {
                        let color = input.read_i32()?;
                        fields.push(("custom_color".to_string(), Tag::Int(color)));
                    }
                    let effects_count = input.read_var_i32()?;
                    for _ in 0..effects_count {
                        let _effect_id = input.read_var_i32()?;
                        let _amplifier = input.read_var_i32()?;
                        let _duration = input.read_var_i32()?;
                        let _ambient = input.read_bool()?;
                        let _show_particles = input.read_bool()?;
                        let _show_icon = input.read_bool()?;
                        let has_hidden = input.read_bool()?;
                        if has_hidden {
                            let _ = input.read_var_i32()?;
                            let _ = input.read_var_i32()?;
                            let _ = input.read_var_i32()?;
                            let _ = input.read_bool()?;
                            let _ = input.read_bool()?;
                            let _ = input.read_bool()?;
                        }
                    }
                    let has_custom_name = input.read_bool()?;
                    if has_custom_name {
                        let custom_name = input.read_utf(32767)?;
                        fields.push(("custom_name".to_string(), Tag::String(custom_name)));
                    }
                    components.set(
                        "minecraft:potion_contents",
                        el_core::item::DataComponent::Generic {
                            type_name: "minecraft:potion_contents".to_string(),
                            value: Tag::Compound(fields),
                        },
                    );
                }
                63 => {
                    let amp = input.read_var_i32()?;
                    components.set(
                        "minecraft:ominous_bottle_amplifier",
                        el_core::item::DataComponent::Generic {
                            type_name: "minecraft:ominous_bottle_amplifier".to_string(),
                            value: Tag::Int(amp),
                        },
                    );
                }
                42 => {
                    let mut fields = Vec::new();
                    let count = input.read_var_i32()?;
                    for _ in 0..count {
                        let ench_id = input.read_var_i32()?;
                        let lvl = input.read_var_i32()?;
                        if let Some(ench_name) = get_enchantment_name_by_id(ench_id - 1) {
                            fields.push((ench_name.to_string(), Tag::Int(lvl)));
                        }
                    }
                    components.set(
                        "minecraft:stored_enchantments",
                        el_core::item::DataComponent::Generic {
                            type_name: "minecraft:stored_enchantments".to_string(),
                            value: Tag::Compound(fields),
                        },
                    );
                }
                61 => {
                    let inst_id = input.read_var_i32()?;
                    if let Some(inst_name) = get_instrument_name_by_id(inst_id - 1) {
                        components.set(
                            "minecraft:instrument",
                            el_core::item::DataComponent::Generic {
                                type_name: "minecraft:instrument".to_string(),
                                value: Tag::String(inst_name.to_string()),
                            },
                        );
                    }
                }
                53 => {
                    let mut effects = Vec::new();
                    let count = input.read_var_i32()?;
                    for _ in 0..count {
                        let effect_id = input.read_var_i32()?;
                        let duration = input.read_var_i32()?;
                        if let Some(eff_name) = get_mob_effect_name_by_id(effect_id - 1) {
                            effects.push(Tag::Compound(vec![
                                ("id".to_string(), Tag::String(eff_name.to_string())),
                                ("duration".to_string(), Tag::Int(duration)),
                            ]));
                        }
                    }
                    components.set(
                        "minecraft:suspicious_stew_effects",
                        el_core::item::DataComponent::Generic {
                            type_name: "minecraft:suspicious_stew_effects".to_string(),
                            value: Tag::List(effects),
                        },
                    );
                }
                _ => {
                    return Err(BufferError::Nbt(format!(
                        "Unsupported component ID {} in ItemStack network protocol",
                        comp_type
                    )));
                }
            }
        }
        let remove_count = input.read_var_i32()?;
        for _ in 0..remove_count {
            let _comp_type = input.read_var_i32()?;
        }
        let mut stack = el_core::ItemStack::new(item, count);
        stack.components = components;
        Ok(stack)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerInput {
    Pickup,
    QuickMove,
    Swap,
    Clone,
    Throw,
    QuickCraft,
    PickupAll,
}
impl ContainerInput {
    pub fn id(self) -> i32 {
        match self {
            ContainerInput::Pickup => 0,
            ContainerInput::QuickMove => 1,
            ContainerInput::Swap => 2,
            ContainerInput::Clone => 3,
            ContainerInput::Throw => 4,
            ContainerInput::QuickCraft => 5,
            ContainerInput::PickupAll => 6,
        }
    }
    pub fn from_id(id: i32) -> Result<Self, BufferError> {
        match id {
            0 => Ok(ContainerInput::Pickup),
            1 => Ok(ContainerInput::QuickMove),
            2 => Ok(ContainerInput::Swap),
            3 => Ok(ContainerInput::Clone),
            4 => Ok(ContainerInput::Throw),
            5 => Ok(ContainerInput::QuickCraft),
            6 => Ok(ContainerInput::PickupAll),
            _ => Err(BufferError::InvalidEnumId {
                name: "ContainerInput",
                id,
            }),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundContainerButtonClickPacket {
    pub container_id: i32,
    pub button_id: i32,
}
impl Packet for ServerboundContainerButtonClickPacket {
    const ID: i32 = 17;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.container_id);
        output.write_var_i32(self.button_id);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            container_id: input.read_var_i32()?,
            button_id: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundContainerClickPacket {
    pub container_id: i32,
    pub state_id: i32,
    pub slot_num: i16,
    pub button_num: i8,
    pub container_input: ContainerInput,
    pub changed_slots: std::collections::BTreeMap<i16, el_core::ItemStack>,
    pub carried_item: el_core::ItemStack,
}
impl Packet for ServerboundContainerClickPacket {
    const ID: i32 = 18;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        if self.changed_slots.len() > 128 {
            return Err(BufferError::CollectionTooLong {
                actual: self.changed_slots.len(),
                max: 128,
            });
        }
        output.write_var_i32(self.container_id);
        output.write_var_i32(self.state_id);
        output.write_i16(self.slot_num);
        output.write_i8(self.button_num);
        output.write_var_i32(self.container_input.id());
        output.write_var_i32(self.changed_slots.len() as i32);
        for (slot, stack) in &self.changed_slots {
            output.write_i16(*slot);
            write_hashed_stack(output, stack)?;
        }
        write_hashed_stack(output, &self.carried_item)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let container_id = input.read_var_i32()?;
        let state_id = input.read_var_i32()?;
        let slot_num = input.read_i16()?;
        let button_num = input.read_i8()?;
        let container_input = ContainerInput::from_id(input.read_var_i32()?)?;
        let changed_count = read_non_negative_var_usize(input)?;
        if changed_count > 128 {
            return Err(BufferError::CollectionTooLong {
                actual: changed_count,
                max: 128,
            });
        }
        let mut changed_slots = std::collections::BTreeMap::new();
        for _ in 0..changed_count {
            let slot = input.read_i16()?;
            let stack = read_hashed_stack(input)?;
            changed_slots.insert(slot, stack);
        }
        Ok(Self {
            container_id,
            state_id,
            slot_num,
            button_num,
            container_input,
            changed_slots,
            carried_item: read_hashed_stack(input)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundContainerClosePacket {
    pub container_id: i32,
}
impl Packet for ServerboundContainerClosePacket {
    const ID: i32 = 19;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.container_id);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            container_id: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundContainerSlotStateChangedPacket {
    pub slot_id: i32,
    pub container_id: i32,
    pub new_state: bool,
}
impl Packet for ServerboundContainerSlotStateChangedPacket {
    const ID: i32 = 20;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.slot_id);
        output.write_var_i32(self.container_id);
        output.write_bool(self.new_state);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            slot_id: input.read_var_i32()?,
            container_id: input.read_var_i32()?,
            new_state: input.read_bool()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundSetCarriedItemPacket {
    pub slot: i16,
}
impl Packet for ServerboundSetCarriedItemPacket {
    const ID: i32 = 53;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i16(self.slot);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            slot: input.read_i16()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServerboundSetCreativeModeSlotPacket {
    pub slot_num: i16,
    pub item_stack: el_core::ItemStack,
}
impl Packet for ServerboundSetCreativeModeSlotPacket {
    const ID: i32 = 56;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i16(self.slot_num);
        write_item_stack(output, &self.item_stack)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            slot_num: input.read_i16()?,
            item_stack: read_item_stack(input)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundContainerClosePacket {
    pub container_id: i32,
}
impl Packet for ClientboundContainerClosePacket {
    const ID: i32 = 17;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.container_id);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            container_id: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundContainerSetContentPacket {
    pub container_id: i32,
    pub state_id: i32,
    pub items: Vec<el_core::ItemStack>,
    pub carried_item: el_core::ItemStack,
}
impl Packet for ClientboundContainerSetContentPacket {
    const ID: i32 = 18;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.container_id);
        output.write_var_i32(self.state_id);
        output.write_var_i32(self.items.len() as i32);
        for stack in &self.items {
            write_item_stack(output, stack)?;
        }
        write_item_stack(output, &self.carried_item)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let container_id = input.read_var_i32()?;
        let state_id = input.read_var_i32()?;
        let item_count = read_non_negative_var_usize(input)?;
        let mut items = Vec::with_capacity(item_count);
        for _ in 0..item_count {
            items.push(read_item_stack(input)?);
        }
        Ok(Self {
            container_id,
            state_id,
            items,
            carried_item: read_item_stack(input)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundContainerSetDataPacket {
    pub container_id: i32,
    pub id: i16,
    pub value: i16,
}
impl Packet for ClientboundContainerSetDataPacket {
    const ID: i32 = 19;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.container_id);
        output.write_i16(self.id);
        output.write_i16(self.value);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            container_id: input.read_var_i32()?,
            id: input.read_i16()?,
            value: input.read_i16()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundContainerSetSlotPacket {
    pub container_id: i32,
    pub state_id: i32,
    pub slot: i16,
    pub item_stack: el_core::ItemStack,
}
impl Packet for ClientboundContainerSetSlotPacket {
    const ID: i32 = 20;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.container_id);
        output.write_var_i32(self.state_id);
        output.write_i16(self.slot);
        write_item_stack(output, &self.item_stack)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            container_id: input.read_var_i32()?,
            state_id: input.read_var_i32()?,
            slot: input.read_i16()?,
            item_stack: read_item_stack(input)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundOpenScreenPacket {
    pub container_id: i32,
    pub menu_type: i32,
    pub title: String,
}
impl Packet for ClientboundOpenScreenPacket {
    const ID: i32 = 59;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.container_id);
        output.write_var_i32(self.menu_type);
        let tag = Tag::String(self.title.clone());
        output.write_nbt_tag(Some(&tag))?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let container_id = input.read_var_i32()?;
        let menu_type = input.read_var_i32()?;
        let title_tag = input
            .read_nbt_tag()?
            .ok_or_else(|| BufferError::Nbt("Expected screen title component".to_string()))?;
        Ok(Self {
            container_id,
            menu_type,
            title: component_tag_to_plain_text(title_tag),
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundSetCursorItemPacket {
    pub contents: el_core::ItemStack,
}
impl Packet for ClientboundSetCursorItemPacket {
    const ID: i32 = 96;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        write_item_stack(output, &self.contents)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            contents: read_item_stack(input)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundSetPlayerInventoryPacket {
    pub slot: i32,
    pub contents: el_core::ItemStack,
}
impl Packet for ClientboundSetPlayerInventoryPacket {
    const ID: i32 = 108;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.slot);
        write_item_stack(output, &self.contents)?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            slot: input.read_var_i32()?,
            contents: read_item_stack(input)?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundBlockUpdatePacket {
    pub pos: ProtoBlockPos,
    pub block_state_id: i32,
}
impl Packet for ClientboundBlockUpdatePacket {
    const ID: i32 = 8;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        write_proto_block_pos(output, &self.pos);
        output.write_var_i32(self.block_state_id);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            pos: read_proto_block_pos(input)?,
            block_state_id: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundBlockEntityDataPacket {
    pub pos: ProtoBlockPos,
    pub block_entity_type_id: i32,
    pub tag: Tag,
}
impl Packet for ClientboundBlockEntityDataPacket {
    const ID: i32 = 6;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        write_proto_block_pos(output, &self.pos);
        output.write_var_i32(self.block_entity_type_id);
        output.write_nbt_tag(Some(&self.tag))
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let pos = read_proto_block_pos(input)?;
        let block_entity_type_id = input.read_var_i32()?;
        let tag = input
            .read_nbt_tag()?
            .ok_or_else(|| BufferError::Nbt("missing block-entity compound tag".to_string()))?;
        if !matches!(tag, Tag::Compound(_)) {
            return Err(BufferError::Nbt(
                "block-entity data root is not a compound".to_string(),
            ));
        }
        Ok(Self {
            pos,
            block_entity_type_id,
            tag,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionBlockUpdate {
    pub pos: ProtoBlockPos,
    pub block_state_id: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundSectionBlocksUpdatePacket {
    pub section_x: i32,
    pub section_y: i32,
    pub section_z: i32,
    pub updates: Vec<SectionBlockUpdate>,
}
fn pack_section_pos(x: i32, y: i32, z: i32) -> i64 {
    ((((x as u64) & 0x3f_ffff) << 42) | (((z as u64) & 0x3f_ffff) << 20) | ((y as u64) & 0x0f_ffff))
        as i64
}
fn unpack_section_pos(packed: i64) -> (i32, i32, i32) {
    let x = (packed >> 42) as i32;
    let y = (packed << 44 >> 44) as i32;
    let z = (packed << 22 >> 42) as i32;
    (x, y, z)
}
impl Packet for ClientboundSectionBlocksUpdatePacket {
    const ID: i32 = 84;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i64(pack_section_pos(
            self.section_x,
            self.section_y,
            self.section_z,
        ));
        if self.updates.len() > 4096 {
            return Err(BufferError::CollectionTooLong {
                actual: self.updates.len(),
                max: 4096,
            });
        }
        output.write_var_i32(self.updates.len() as i32);
        for update in &self.updates {
            let relative_x = update.pos.x - (self.section_x << 4);
            let relative_y = update.pos.y - (self.section_y << 4);
            let relative_z = update.pos.z - (self.section_z << 4);
            if !(0..16).contains(&relative_x)
                || !(0..16).contains(&relative_y)
                || !(0..16).contains(&relative_z)
                || update.block_state_id < 0
            {
                return Err(BufferError::InvalidEnumId {
                    name: "section block update",
                    id: update.block_state_id,
                });
            }
            let relative = (((relative_x & 15) as u64) << 8)
                | (((relative_z & 15) as u64) << 4)
                | ((relative_y & 15) as u64);
            let packed_change = ((update.block_state_id as u64) << 12) | relative;
            output.write_var_i64(packed_change as i64);
        }
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let (section_x, section_y, section_z) = unpack_section_pos(input.read_i64()?);
        let count = input.read_var_i32()?;
        if count < 0 {
            return Err(BufferError::NegativeLength);
        }
        if count > 4096 {
            return Err(BufferError::CollectionTooLong {
                actual: count as usize,
                max: 4096,
            });
        }
        let mut updates = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let packed_change = input.read_var_i64()? as u64;
            let relative = packed_change & 0xfff;
            let state_id = packed_change >> 12;
            if state_id > i32::MAX as u64 {
                return Err(BufferError::InvalidEnumId {
                    name: "block state",
                    id: -1,
                });
            }
            updates.push(SectionBlockUpdate {
                pos: ProtoBlockPos {
                    x: (section_x << 4) + ((relative >> 8) & 15) as i32,
                    y: (section_y << 4) + (relative & 15) as i32,
                    z: (section_z << 4) + ((relative >> 4) & 15) as i32,
                },
                block_state_id: state_id as i32,
            });
        }
        Ok(Self {
            section_x,
            section_y,
            section_z,
            updates,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientboundBlockEventPacket {
    pub pos: ProtoBlockPos,
    pub b0: u8,
    pub b1: u8,
    pub block: i32,
}
impl Packet for ClientboundBlockEventPacket {
    const ID: i32 = 7;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        write_proto_block_pos(output, &self.pos);
        output.write_u8(self.b0);
        output.write_u8(self.b1);
        output.write_var_i32(self.block);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            pos: read_proto_block_pos(input)?,
            b0: input.read_u8()?,
            b1: input.read_u8()?,
            block: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClientboundLevelEventPacket {
    pub type_: i32,
    pub pos: ProtoBlockPos,
    pub data: i32,
    pub global_event: bool,
}
impl Packet for ClientboundLevelEventPacket {
    const ID: i32 = 25;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i32(self.type_);
        write_proto_block_pos(output, &self.pos);
        output.write_i32(self.data);
        output.write_bool(self.global_event);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            type_: input.read_i32()?,
            pos: read_proto_block_pos(input)?,
            data: input.read_i32()?,
            global_event: input.read_bool()?,
        })
    }
}
pub const SOUND_LOCATION_ACCURACY: f64 = 8.0;
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundSoundPacket {
    pub sound: String,
    pub fixed_range: Option<f32>,
    pub source: i32,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub volume: f32,
    pub pitch: f32,
    pub seed: i64,
}
impl ClientboundSoundPacket {
    #[allow(clippy::too_many_arguments)]
    pub fn at(
        sound: String,
        fixed_range: Option<f32>,
        source: i32,
        x: f64,
        y: f64,
        z: f64,
        volume: f32,
        pitch: f32,
        seed: i64,
    ) -> Self {
        Self {
            sound,
            fixed_range,
            source,
            x: (x * SOUND_LOCATION_ACCURACY) as i32,
            y: (y * SOUND_LOCATION_ACCURACY) as i32,
            z: (z * SOUND_LOCATION_ACCURACY) as i32,
            volume,
            pitch,
            seed,
        }
    }
    pub fn position(&self) -> (f64, f64, f64) {
        (
            self.x as f64 / SOUND_LOCATION_ACCURACY,
            self.y as f64 / SOUND_LOCATION_ACCURACY,
            self.z as f64 / SOUND_LOCATION_ACCURACY,
        )
    }
}
impl Packet for ClientboundSoundPacket {
    const ID: i32 = 117;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(0);
        output.write_utf(&self.sound, 32767)?;
        match self.fixed_range {
            Some(range) => {
                output.write_bool(true);
                output.write_f32(range);
            }
            None => output.write_bool(false),
        }
        output.write_var_i32(self.source);
        output.write_i32(self.x);
        output.write_i32(self.y);
        output.write_i32(self.z);
        output.write_f32(self.volume);
        output.write_f32(self.pitch);
        output.write_i64(self.seed);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let holder_id = input.read_var_i32()?;
        if holder_id != 0 {
            return Err(BufferError::InvalidEnumId {
                name: "SoundEvent holder",
                id: holder_id,
            });
        }
        let sound = input.read_utf(32767)?;
        let fixed_range = if input.read_bool()? {
            Some(input.read_f32()?)
        } else {
            None
        };
        Ok(Self {
            sound,
            fixed_range,
            source: input.read_var_i32()?,
            x: input.read_i32()?,
            y: input.read_i32()?,
            z: input.read_i32()?,
            volume: input.read_f32()?,
            pitch: input.read_f32()?,
            seed: input.read_i64()?,
        })
    }
}
fn write_hashed_stack(
    output: &mut PacketBuffer,
    stack: &el_core::ItemStack,
) -> Result<(), BufferError> {
    if stack.is_empty() {
        output.write_bool(false);
    } else {
        output.write_bool(true);
        let item_id =
            el_core::registry::BuiltInRegistries::items().get_id(&stack.item.location) as i32;
        output.write_var_i32(item_id);
        output.write_var_i32(stack.count);
        output.write_var_i32(0);
        output.write_var_i32(0);
    }
    Ok(())
}
fn read_hashed_stack(input: &mut PacketBuffer) -> Result<el_core::ItemStack, BufferError> {
    if !input.read_bool()? {
        return Ok(el_core::ItemStack::empty());
    }
    let item_id = input.read_var_i32()?;
    let item = el_core::registry::BuiltInRegistries::items()
        .get_by_id(item_id as usize)
        .clone();
    let count = input.read_var_i32()?;
    let added_count = read_non_negative_var_usize(input)?;
    if added_count > 256 {
        return Err(BufferError::CollectionTooLong {
            actual: added_count,
            max: 256,
        });
    }
    for _ in 0..added_count {
        let _component_type = input.read_var_i32()?;
        let _hash = input.read_i32()?;
    }
    let removed_count = read_non_negative_var_usize(input)?;
    if removed_count > 256 {
        return Err(BufferError::CollectionTooLong {
            actual: removed_count,
            max: 256,
        });
    }
    for _ in 0..removed_count {
        let _component_type = input.read_var_i32()?;
    }
    Ok(el_core::ItemStack::new(item, count))
}
fn write_entity_data_value(
    output: &mut PacketBuffer,
    type_id: i32,
    val: &el_core::entity::EntityDataValue,
) -> Result<(), BufferError> {
    output.write_var_i32(type_id);
    match val {
        el_core::entity::EntityDataValue::Byte(v) => output.write_i8(*v),
        el_core::entity::EntityDataValue::VarInt(v) => output.write_var_i32(*v),
        el_core::entity::EntityDataValue::VarLong(v) => output.write_var_i64(*v),
        el_core::entity::EntityDataValue::Float(v) => output.write_f32(*v),
        el_core::entity::EntityDataValue::String(v) => output.write_utf(v, 32767)?,
        el_core::entity::EntityDataValue::Component(v) => output.write_utf(v, 262144)?,
        el_core::entity::EntityDataValue::OptionalComponent(v) => {
            output.write_bool(v.is_some());
            if let Some(s) = v {
                output.write_utf(s, 262144)?;
            }
        }
        el_core::entity::EntityDataValue::ItemStack(v) => {
            write_item_stack(output, v)?;
        }
        el_core::entity::EntityDataValue::Boolean(v) => output.write_bool(*v),
        el_core::entity::EntityDataValue::Rotations(x, y, z) => {
            output.write_f32(*x);
            output.write_f32(*y);
            output.write_f32(*z);
        }
        el_core::entity::EntityDataValue::BlockPos(pos) => {
            let val = (((pos.x as i64) & 0x3FFFFFF) << 38)
                | (((pos.z as i64) & 0x3FFFFFF) << 12)
                | ((pos.y as i64) & 0xFFF);
            output.write_i64(val);
        }
        el_core::entity::EntityDataValue::OptionalBlockPos(pos) => {
            output.write_bool(pos.is_some());
            if let Some(p) = pos {
                let val = (((p.x as i64) & 0x3FFFFFF) << 38)
                    | (((p.z as i64) & 0x3FFFFFF) << 12)
                    | ((p.y as i64) & 0xFFF);
                output.write_i64(val);
            }
        }
        el_core::entity::EntityDataValue::Direction(v) => output.write_var_i32(*v),
        el_core::entity::EntityDataValue::OptionalUUID(v) => {
            output.write_bool(v.is_some());
            if let Some(u) = v {
                output.write_uuid_bytes(&u.into_bytes());
            }
        }
        el_core::entity::EntityDataValue::BlockState(v) => output.write_var_i32(*v),
        el_core::entity::EntityDataValue::OptionalBlockState(v) => {
            output.write_var_i32(v.unwrap_or(0));
        }
        el_core::entity::EntityDataValue::NbtCompound(v) => {
            output.write_nbt_tag(Some(v))?;
        }
        el_core::entity::EntityDataValue::OptionalVarInt(v) => {
            output.write_var_i32(v.map(|x| x + 1).unwrap_or(0));
        }
        el_core::entity::EntityDataValue::Pose(v) => output.write_var_i32(*v),
    }
    Ok(())
}
fn read_entity_data_value(
    input: &mut PacketBuffer,
    type_id: i32,
) -> Result<el_core::entity::EntityDataValue, BufferError> {
    match type_id {
        0 => Ok(el_core::entity::EntityDataValue::Byte(input.read_i8()?)),
        1 => Ok(el_core::entity::EntityDataValue::VarInt(
            input.read_var_i32()?,
        )),
        2 => Ok(el_core::entity::EntityDataValue::VarLong(
            input.read_var_i64()?,
        )),
        3 => Ok(el_core::entity::EntityDataValue::Float(input.read_f32()?)),
        4 => Ok(el_core::entity::EntityDataValue::String(
            input.read_utf(32767)?,
        )),
        5 => Ok(el_core::entity::EntityDataValue::Component(
            input.read_utf(262144)?,
        )),
        6 => {
            let has_val = input.read_bool()?;
            let val = if has_val {
                Some(input.read_utf(262144)?)
            } else {
                None
            };
            Ok(el_core::entity::EntityDataValue::OptionalComponent(val))
        }
        7 => Ok(el_core::entity::EntityDataValue::ItemStack(
            read_item_stack(input)?,
        )),
        8 => Ok(el_core::entity::EntityDataValue::Boolean(
            input.read_bool()?,
        )),
        9 => {
            let x = input.read_f32()?;
            let y = input.read_f32()?;
            let z = input.read_f32()?;
            Ok(el_core::entity::EntityDataValue::Rotations(x, y, z))
        }
        10 => {
            let val = input.read_i64()?;
            let x = (val >> 38) as i32;
            let z = ((val >> 12) & 0x3FFFFFF) as i32;
            let mut y = (val & 0xFFF) as i32;
            if y >= 2048 {
                y -= 4096;
            }
            Ok(el_core::entity::EntityDataValue::BlockPos(
                el_math::block_pos::BlockPos { x, y, z },
            ))
        }
        11 => {
            let has_val = input.read_bool()?;
            let pos = if has_val {
                let val = input.read_i64()?;
                let x = (val >> 38) as i32;
                let z = ((val >> 12) & 0x3FFFFFF) as i32;
                let mut y = (val & 0xFFF) as i32;
                if y >= 2048 {
                    y -= 4096;
                }
                Some(el_math::block_pos::BlockPos { x, y, z })
            } else {
                None
            };
            Ok(el_core::entity::EntityDataValue::OptionalBlockPos(pos))
        }
        12 => Ok(el_core::entity::EntityDataValue::Direction(
            input.read_var_i32()?,
        )),
        13 => {
            let has_val = input.read_bool()?;
            let u = if has_val {
                Some(uuid::Uuid::from_bytes(input.read_uuid_bytes()?))
            } else {
                None
            };
            Ok(el_core::entity::EntityDataValue::OptionalUUID(u))
        }
        14 => Ok(el_core::entity::EntityDataValue::BlockState(
            input.read_var_i32()?,
        )),
        15 => {
            let state = input.read_var_i32()?;
            Ok(el_core::entity::EntityDataValue::OptionalBlockState(
                if state == 0 { None } else { Some(state) },
            ))
        }
        16 => {
            let tag = input
                .read_nbt_tag()?
                .ok_or_else(|| BufferError::Nbt("Expected non-empty tag".to_string()))?;
            Ok(el_core::entity::EntityDataValue::NbtCompound(tag))
        }
        19 => {
            let val = input.read_var_i32()?;
            Ok(el_core::entity::EntityDataValue::OptionalVarInt(
                if val == 0 { None } else { Some(val - 1) },
            ))
        }
        20 => Ok(el_core::entity::EntityDataValue::Pose(
            input.read_var_i32()?,
        )),
        _ => Err(BufferError::InvalidEnumId {
            name: "EntityDataValueSerializer",
            id: type_id,
        }),
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundAddEntityPacket {
    pub id: i32,
    pub uuid: uuid::Uuid,
    pub entity_type: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pitch: i8,
    pub yaw: i8,
    pub head_yaw: i8,
    pub data: i32,
    pub xa: i16,
    pub ya: i16,
    pub za: i16,
}
impl Packet for ClientboundAddEntityPacket {
    const ID: i32 = 0x01;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.id);
        output.write_uuid_bytes(&self.uuid.into_bytes());
        output.write_var_i32(self.entity_type);
        output.write_f64(self.x);
        output.write_f64(self.y);
        output.write_f64(self.z);
        output.write_i8(self.pitch);
        output.write_i8(self.yaw);
        output.write_i8(self.head_yaw);
        output.write_var_i32(self.data);
        output.write_i16(self.xa);
        output.write_i16(self.ya);
        output.write_i16(self.za);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            id: input.read_var_i32()?,
            uuid: uuid::Uuid::from_bytes(input.read_uuid_bytes()?),
            entity_type: input.read_var_i32()?,
            x: input.read_f64()?,
            y: input.read_f64()?,
            z: input.read_f64()?,
            pitch: input.read_i8()?,
            yaw: input.read_i8()?,
            head_yaw: input.read_i8()?,
            data: input.read_var_i32()?,
            xa: input.read_i16()?,
            ya: input.read_i16()?,
            za: input.read_i16()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundRemoveEntitiesPacket {
    pub entity_ids: Vec<i32>,
}
impl Packet for ClientboundRemoveEntitiesPacket {
    const ID: i32 = 0x40;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.entity_ids.len() as i32);
        for id in &self.entity_ids {
            output.write_var_i32(*id);
        }
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let count = input.read_var_i32()?;
        if count < 0 {
            return Err(BufferError::NegativeLength);
        }
        let mut entity_ids = Vec::with_capacity(count as usize);
        for _ in 0..count {
            entity_ids.push(input.read_var_i32()?);
        }
        Ok(Self { entity_ids })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundSetEntityDataPacket {
    pub entity_id: i32,
    pub values: Vec<(u8, i32, el_core::entity::EntityDataValue)>,
}
impl Packet for ClientboundSetEntityDataPacket {
    const ID: i32 = 0x56;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.entity_id);
        for (index, serializer_id, value) in &self.values {
            output.write_u8(*index);
            write_entity_data_value(output, *serializer_id, value)?;
        }
        output.write_u8(0xFF);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let entity_id = input.read_var_i32()?;
        let mut values = Vec::new();
        loop {
            let index = input.read_u8()?;
            if index == 0xFF {
                break;
            }
            let serializer_id = input.read_var_i32()?;
            let value = read_entity_data_value(input, serializer_id)?;
            values.push((index, serializer_id, value));
        }
        Ok(Self { entity_id, values })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundTeleportEntityPacket {
    pub entity_id: i32,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: i8,
    pub pitch: i8,
    pub on_ground: bool,
}
impl Packet for ClientboundTeleportEntityPacket {
    const ID: i32 = 0x6D;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.entity_id);
        output.write_f64(self.x);
        output.write_f64(self.y);
        output.write_f64(self.z);
        output.write_i8(self.yaw);
        output.write_i8(self.pitch);
        output.write_bool(self.on_ground);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            entity_id: input.read_var_i32()?,
            x: input.read_f64()?,
            y: input.read_f64()?,
            z: input.read_f64()?,
            yaw: input.read_i8()?,
            pitch: input.read_i8()?,
            on_ground: input.read_bool()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundMoveEntityPosRotPacket {
    pub entity_id: i32,
    pub delta_x: i16,
    pub delta_y: i16,
    pub delta_z: i16,
    pub yaw: i8,
    pub pitch: i8,
    pub on_ground: bool,
}
impl Packet for ClientboundMoveEntityPosRotPacket {
    const ID: i32 = 0x2A;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.entity_id);
        output.write_i16(self.delta_x);
        output.write_i16(self.delta_y);
        output.write_i16(self.delta_z);
        output.write_i8(self.yaw);
        output.write_i8(self.pitch);
        output.write_bool(self.on_ground);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            entity_id: input.read_var_i32()?,
            delta_x: input.read_i16()?,
            delta_y: input.read_i16()?,
            delta_z: input.read_i16()?,
            yaw: input.read_i8()?,
            pitch: input.read_i8()?,
            on_ground: input.read_bool()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundSetHealthPacket {
    pub health: f32,
    pub food: i32,
    pub saturation: f32,
}
impl Packet for ClientboundSetHealthPacket {
    const ID: i32 = 104;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_f32(self.health);
        output.write_var_i32(self.food);
        output.write_f32(self.saturation);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            health: input.read_f32()?,
            food: input.read_var_i32()?,
            saturation: input.read_f32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundSetExperiencePacket {
    pub experience_progress: f32,
    pub total_experience: i32,
    pub experience_level: i32,
}
impl Packet for ClientboundSetExperiencePacket {
    const ID: i32 = 103;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_f32(self.experience_progress);
        output.write_var_i32(self.total_experience);
        output.write_var_i32(self.experience_level);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            experience_progress: input.read_f32()?,
            total_experience: input.read_var_i32()?,
            experience_level: input.read_var_i32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClientboundDisconnectPacket {
    pub reason: String,
}
impl Packet for ClientboundDisconnectPacket {
    const ID: i32 = 32;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        let tag = Tag::String(self.reason.clone());
        output.write_nbt_tag(Some(&tag))?;
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        let tag = input
            .read_nbt_tag()?
            .ok_or_else(|| BufferError::Nbt("Expected non-empty tag".to_string()))?;
        let reason = match &tag {
            Tag::String(s) => s.clone(),
            Tag::Compound(pairs) => pairs
                .iter()
                .find(|(k, _)| k == "text")
                .and_then(|(_, v)| {
                    if let Tag::String(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| format!("{:?}", tag)),
            _ => format!("{:?}", tag),
        };
        Ok(Self { reason })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundGameEventPacket {
    pub event_id: u8,
    pub value: f32,
}
impl Packet for ClientboundGameEventPacket {
    const ID: i32 = 38;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i8(self.event_id as i8);
        output.write_f32(self.value);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            event_id: input.read_i8()? as u8,
            value: input.read_f32()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundRespawnPacket {
    pub spawn_info: CommonPlayerSpawnInfo,
    pub data_to_keep: u8,
}
impl Packet for ClientboundRespawnPacket {
    const ID: i32 = 82;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        self.spawn_info.encode(output)?;
        output.write_i8(self.data_to_keep as i8);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            spawn_info: CommonPlayerSpawnInfo::decode(input)?,
            data_to_keep: input.read_i8()? as u8,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClientboundChangeDifficultyPacket {
    pub difficulty: u8,
    pub locked: bool,
}
impl Packet for ClientboundChangeDifficultyPacket {
    const ID: i32 = 10;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_i8(self.difficulty as i8);
        output.write_bool(self.locked);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            difficulty: input.read_i8()? as u8,
            locked: input.read_bool()?,
        })
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundClientCommandPacket {
    pub action: i32,
}
impl Packet for ServerboundClientCommandPacket {
    const ID: i32 = 12;
    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_var_i32(self.action);
        Ok(())
    }
    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            action: input.read_var_i32()?,
        })
    }
}
