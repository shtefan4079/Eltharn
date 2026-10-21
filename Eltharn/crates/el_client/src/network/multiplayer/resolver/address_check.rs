#![allow(dead_code)]

use crate::network::multiplayer::resolver::resolved_server_address::ResolvedServerAddress;
use crate::network::multiplayer::resolver::server_address::ServerAddress;

pub trait AddressCheck {
    fn is_resolved_address_allowed(&self, address: &ResolvedServerAddress) -> bool;
    fn is_address_allowed(&self, address: &ServerAddress) -> bool;
}

pub struct AllowAllAddressCheck;

impl AddressCheck for AllowAllAddressCheck {
    fn is_resolved_address_allowed(&self, _address: &ResolvedServerAddress) -> bool {
        true
    }

    fn is_address_allowed(&self, _address: &ServerAddress) -> bool {
        true
    }
}
