#![allow(dead_code)]

use crate::network::multiplayer::resolver::resolved_server_address::ResolvedServerAddress;
use crate::network::multiplayer::resolver::server_address::ServerAddress;
use std::net::ToSocketAddrs;

pub trait ServerAddressResolver {
    fn resolve(&self, address: &ServerAddress) -> Option<ResolvedServerAddress>;
}

pub struct SystemServerAddressResolver;

impl ServerAddressResolver for SystemServerAddressResolver {
    fn resolve(&self, address: &ServerAddress) -> Option<ResolvedServerAddress> {
        let host = address.host();
        let addr_str = if host.contains(':') {
            format!("[{}]:{}", host, address.port())
        } else {
            format!("{}:{}", host, address.port())
        };
        if let Ok(mut addrs) = addr_str.to_socket_addrs() {
            if let Some(socket_addr) = addrs.next() {
                let host_ip = socket_addr.ip().to_string();
                return Some(ResolvedServerAddress::new(
                    address.host().to_string(),
                    host_ip,
                    address.port(),
                    socket_addr,
                ));
            }
        }
        None
    }
}
