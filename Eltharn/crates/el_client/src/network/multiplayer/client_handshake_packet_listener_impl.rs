#![allow(dead_code)]

use crate::network::multiplayer::server_data::ServerData;

pub struct ClientHandshakePacketListenerImpl {
    pub server_data: Option<ServerData>,
    pub was_transferred_to: bool,
}

impl ClientHandshakePacketListenerImpl {
    pub fn new(server_data: Option<ServerData>, was_transferred_to: bool) -> Self {
        Self {
            server_data,
            was_transferred_to,
        }
    }
}
