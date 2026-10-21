use el_core::registry::{MappedRegistry, RegistryAccess};
use el_protocol::packets::configuration::{
    ClientboundFinishConfigurationPacket, ClientboundRegistryDataPacket,
    ClientboundSelectKnownPacksPacket, ServerboundFinishConfigurationPacket,
    ServerboundSelectKnownPacksPacket,
};
use el_protocol::packets::handshake::{ClientIntent, ClientIntentionPacket};
use el_protocol::packets::login::{
    ClientboundLoginFinishedPacket, ServerboundHelloPacket, ServerboundLoginAcknowledgedPacket,
};
use el_protocol::{DecodeError, Packet, PacketBuffer, decode_var_i32, read_frame, write_frame};
use std::io::{self, Read, Write};
use std::sync::mpsc::{self, Receiver, Sender};
pub struct LocalChannel {
    rx: Receiver<Vec<u8>>,
    tx: Sender<Vec<u8>>,
    buf: Vec<u8>,
    buf_pos: usize,
}
impl LocalChannel {
    pub fn pair() -> (LocalChannel, LocalChannel) {
        let (tx_a, rx_b) = mpsc::channel::<Vec<u8>>();
        let (tx_b, rx_a) = mpsc::channel::<Vec<u8>>();
        let a = LocalChannel {
            rx: rx_a,
            tx: tx_a,
            buf: Vec::new(),
            buf_pos: 0,
        };
        let b = LocalChannel {
            rx: rx_b,
            tx: tx_b,
            buf: Vec::new(),
            buf_pos: 0,
        };
        (a, b)
    }
    pub fn try_read_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        if self.buf_pos < self.buf.len() {
            let chunk = self.buf[self.buf_pos..].to_vec();
            self.buf_pos = self.buf.len();
            return Ok(Some(chunk));
        }
        match self.rx.try_recv() {
            Ok(chunk) => Ok(Some(chunk)),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Ok(None),
        }
    }
}
impl Read for LocalChannel {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        while self.buf_pos >= self.buf.len() {
            match self.rx.recv() {
                Ok(chunk) => {
                    self.buf = chunk;
                    self.buf_pos = 0;
                }
                Err(_) => return Ok(0),
            }
        }
        let available = self.buf.len() - self.buf_pos;
        let to_copy = buf.len().min(available);
        buf[..to_copy].copy_from_slice(&self.buf[self.buf_pos..self.buf_pos + to_copy]);
        self.buf_pos += to_copy;
        Ok(to_copy)
    }
}
impl Write for LocalChannel {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.tx
            .send(buf.to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "channel closed"))?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
#[derive(Debug)]
pub enum LocalConnectionError {
    Io(io::Error),
    Protocol(el_protocol::BufferError),
    Frame(el_protocol::FrameError),
    UnexpectedPacketId { expected: i32, actual: i32 },
}
impl From<io::Error> for LocalConnectionError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<el_protocol::BufferError> for LocalConnectionError {
    fn from(e: el_protocol::BufferError) -> Self {
        Self::Protocol(e)
    }
}
impl From<el_protocol::FrameError> for LocalConnectionError {
    fn from(e: el_protocol::FrameError) -> Self {
        Self::Frame(e)
    }
}
impl std::fmt::Display for LocalConnectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl std::error::Error for LocalConnectionError {}
pub enum ConnectionBackend {
    Local { channel: LocalChannel },
}
pub struct LocalConnection {
    backend: ConnectionBackend,
    incoming_frame_bytes: Vec<u8>,
}
const PROTOCOL_VERSION: i32 = 1_073_742_146;
const MAX_FRAME: usize = 2 * 1024 * 1024;
impl LocalConnection {
    pub fn connect() -> Self {
        let (client_end, _server_end) = LocalChannel::pair();
        LocalConnection {
            backend: ConnectionBackend::Local {
                channel: client_end,
            },
            incoming_frame_bytes: Vec::new(),
        }
    }
    pub fn send_packet<P: Packet>(&mut self, packet: &P) -> Result<(), LocalConnectionError> {
        let mut buf = PacketBuffer::new();
        buf.write_var_i32(P::ID);
        packet.encode(&mut buf)?;
        let payload_bytes = buf.into_inner();
        self.send_raw_payload(&payload_bytes)
    }
    pub fn send_raw_payload(&mut self, payload_bytes: &[u8]) -> Result<(), LocalConnectionError> {
        match &mut self.backend {
            ConnectionBackend::Local { channel } => {
                write_frame(channel, payload_bytes)?;
            }
        }
        Ok(())
    }
    pub fn read_packet_raw(&mut self) -> Result<(i32, Vec<u8>), LocalConnectionError> {
        if let Some(packet) = self.pop_buffered_packet()? {
            return Ok(packet);
        }
        match &mut self.backend {
            ConnectionBackend::Local { channel } => {
                let frame = read_frame(channel, MAX_FRAME)?;
                let mut buf = PacketBuffer::from_bytes(frame);
                let id = buf.read_var_i32()?;
                let payload = buf.read_bytes(buf.remaining())?;
                Ok((id, payload))
            }
        }
    }
    pub fn try_read_packet_raw(&mut self) -> Result<Option<(i32, Vec<u8>)>, LocalConnectionError> {
        match &mut self.backend {
            ConnectionBackend::Local { channel } => {
                while let Some(chunk) = channel.try_read_chunk()? {
                    self.incoming_frame_bytes.extend_from_slice(&chunk);
                }
                self.pop_buffered_packet()
            }
        }
    }
    fn pop_buffered_packet(&mut self) -> Result<Option<(i32, Vec<u8>)>, LocalConnectionError> {
        let Some(frame) = self.pop_buffered_frame()? else {
            return Ok(None);
        };
        let mut buf = PacketBuffer::from_bytes(frame);
        let id = buf.read_var_i32()?;
        let payload = buf.read_bytes(buf.remaining())?;
        Ok(Some((id, payload)))
    }
    fn pop_buffered_frame(&mut self) -> Result<Option<Vec<u8>>, LocalConnectionError> {
        if self.incoming_frame_bytes.is_empty() {
            return Ok(None);
        }
        let (length, header_len) = match decode_var_i32(&self.incoming_frame_bytes) {
            Ok(decoded) => decoded,
            Err(DecodeError::Incomplete) => return Ok(None),
            Err(error) => {
                return Err(LocalConnectionError::Frame(
                    el_protocol::FrameError::VarInt(error),
                ));
            }
        };
        if length < 0 {
            return Err(LocalConnectionError::Frame(
                el_protocol::FrameError::NegativeLength(length),
            ));
        }
        let length = length as usize;
        if length > MAX_FRAME {
            return Err(LocalConnectionError::Frame(
                el_protocol::FrameError::TooLarge {
                    length,
                    max: MAX_FRAME,
                },
            ));
        }
        let frame_end = header_len + length;
        if self.incoming_frame_bytes.len() < frame_end {
            return Ok(None);
        }
        let frame = self.incoming_frame_bytes[header_len..frame_end].to_vec();
        self.incoming_frame_bytes.drain(..frame_end);
        Ok(Some(frame))
    }
    pub fn read_packet<P: Packet>(&mut self) -> Result<P, LocalConnectionError> {
        let (id, payload) = self.read_packet_raw()?;
        if id != P::ID {
            return Err(LocalConnectionError::UnexpectedPacketId {
                expected: P::ID,
                actual: id,
            });
        }
        let mut buf = PacketBuffer::from_bytes(payload);
        Ok(P::decode(&mut buf)?)
    }
    pub fn login(
        &mut self,
        username: &str,
    ) -> Result<ClientboundLoginFinishedPacket, LocalConnectionError> {
        let handshake = ClientIntentionPacket {
            protocol_version: PROTOCOL_VERSION,
            host_name: "localhost".to_string(),
            port: 25565,
            intention: ClientIntent::Login,
        };
        self.send_packet(&handshake)?;
        let hello = ServerboundHelloPacket {
            name: username.to_string(),
            profile_id: offline_uuid(username),
        };
        self.send_packet(&hello)?;
        let login_finished = self.read_packet::<ClientboundLoginFinishedPacket>()?;
        self.send_packet(&ServerboundLoginAcknowledgedPacket)?;
        Ok(login_finished)
    }
    pub fn configure(
        &mut self,
        registry_access: &mut RegistryAccess,
    ) -> Result<(), LocalConnectionError> {
        loop {
            let (id, payload) = self.read_packet_raw()?;
            if id == ClientboundFinishConfigurationPacket::ID {
                self.send_packet(&ServerboundFinishConfigurationPacket)?;
                break;
            }
            if id == ClientboundSelectKnownPacksPacket::ID {
                let mut buf = PacketBuffer::from_bytes(payload);
                let clientbound_packs = ClientboundSelectKnownPacksPacket::decode(&mut buf)?;
                let response = ServerboundSelectKnownPacksPacket {
                    known_packs: clientbound_packs.known_packs,
                };
                self.send_packet(&response)?;
            } else if id == ClientboundRegistryDataPacket::ID {
                let mut buf = PacketBuffer::from_bytes(payload);
                let reg_packet = ClientboundRegistryDataPacket::decode(&mut buf)?;
                let reg_loc = el_core::ResourceLocation::from_string(&reg_packet.registry);
                let mut mapped_reg = MappedRegistry::<String>::new(reg_loc.clone());
                for entry in reg_packet.entries {
                    let entry_loc = el_core::ResourceLocation::from_string(&entry.id);
                    mapped_reg.register(entry_loc, entry.id.clone());
                }
                registry_access.add_registry(reg_loc, mapped_reg);
            }
        }
        Ok(())
    }
}
fn offline_uuid(username: &str) -> [u8; 16] {
    *el_protocol_backend::offline_uuid(username).as_bytes()
}
