#![allow(dead_code)]

use crate::network::multiplayer::client_common_packet_listener_impl::ClientCommonPacketListenerImpl;
use crate::network::multiplayer::common_listener_cookie::CommonListenerCookie;
use crate::network::multiplayer::registry_data_collector::RegistryDataCollector;
use el_core::registry::RegistryAccess;

pub struct ClientConfigurationPacketListenerImpl {
    pub common_source: ClientCommonPacketListenerImpl,
    pub received_registries: RegistryAccess,
    pub registry_data_collector: RegistryDataCollector,
}

impl ClientConfigurationPacketListenerImpl {
    pub fn new(cookie: CommonListenerCookie) -> Self {
        let received_registries = RegistryAccess::new();
        Self {
            common_source: ClientCommonPacketListenerImpl::new(cookie),
            received_registries,
            registry_data_collector: RegistryDataCollector::new(),
        }
    }
}
