#![allow(dead_code)]

use el_core::registry::RegistryAccess;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClientRegistryLayer {
    Static,
    Remote,
}

impl ClientRegistryLayer {
    pub fn create_registry_access() -> RegistryAccess {
        RegistryAccess::new()
    }
}
