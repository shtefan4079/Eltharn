#![allow(dead_code)]

use std::net::SocketAddr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedServerAddress {
    pub host_name: String,
    pub host_ip: String,
    pub port: u16,
    pub socket_addr: SocketAddr,
}

impl ResolvedServerAddress {
    pub fn new(host_name: String, host_ip: String, port: u16, socket_addr: SocketAddr) -> Self {
        Self {
            host_name,
            host_ip,
            port,
            socket_addr,
        }
    }

    pub fn host_name(&self) -> &str {
        &self.host_name
    }

    pub fn host_ip(&self) -> &str {
        &self.host_ip
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn as_socket_addr(&self) -> SocketAddr {
        self.socket_addr
    }
}
