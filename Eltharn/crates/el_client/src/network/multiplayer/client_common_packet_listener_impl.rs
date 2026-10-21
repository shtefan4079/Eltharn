#![allow(dead_code)]

use crate::network::multiplayer::common_listener_cookie::CommonListenerCookie;
use crate::network::multiplayer::server_data::ServerData;

pub struct ClientCommonPacketListenerImpl {
    pub server_data: Option<ServerData>,
    pub server_brand: Option<String>,
    pub seen_insecure_chat_warning: bool,
}

impl ClientCommonPacketListenerImpl {
    pub fn new(cookie: CommonListenerCookie) -> Self {
        Self {
            server_data: cookie.server_data,
            server_brand: cookie.server_brand,
            seen_insecure_chat_warning: cookie.seen_insecure_chat_warning,
        }
    }
}
