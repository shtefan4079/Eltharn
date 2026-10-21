use crate::{BufferError, Packet, PacketBuffer};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServerboundClientInformationPacket {
    pub language: String,
    pub view_distance: i8,
    pub chat_visibility: i32,
    pub chat_colors: bool,
    pub model_customization: u8,
    pub main_hand: i32,
    pub text_filtering_enabled: bool,
    pub allows_listing: bool,
    pub particle_status: i32,
}

impl Packet for ServerboundClientInformationPacket {
    const ID: i32 = 0x0E;

    fn encode(&self, output: &mut PacketBuffer) -> Result<(), BufferError> {
        output.write_utf(&self.language, 16)?;
        output.write_i8(self.view_distance);
        output.write_var_i32(self.chat_visibility);
        output.write_bool(self.chat_colors);
        output.write_u8(self.model_customization);
        output.write_var_i32(self.main_hand);
        output.write_bool(self.text_filtering_enabled);
        output.write_bool(self.allows_listing);
        output.write_var_i32(self.particle_status);
        Ok(())
    }

    fn decode(input: &mut PacketBuffer) -> Result<Self, BufferError> {
        Ok(Self {
            language: input.read_utf(16)?,
            view_distance: input.read_i8()?,
            chat_visibility: input.read_var_i32()?,
            chat_colors: input.read_bool()?,
            model_customization: input.read_u8()?,
            main_hand: input.read_var_i32()?,
            text_filtering_enabled: input.read_bool()?,
            allows_listing: input.read_bool()?,
            particle_status: input.read_var_i32()?,
        })
    }
}
