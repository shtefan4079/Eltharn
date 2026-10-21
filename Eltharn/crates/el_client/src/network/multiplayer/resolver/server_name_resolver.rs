#![allow(dead_code)]
use crate::network::multiplayer::resolver::address_check::{AddressCheck, AllowAllAddressCheck};
use crate::network::multiplayer::resolver::resolved_server_address::ResolvedServerAddress;
use crate::network::multiplayer::resolver::server_address::ServerAddress;
use crate::network::multiplayer::resolver::server_address_resolver::{
    ServerAddressResolver, SystemServerAddressResolver,
};
use crate::network::multiplayer::resolver::server_redirect_handler::{
    ServerRedirectHandler, create_dns_srv_redirect_handler,
};
pub struct ServerNameResolver {
    resolver: Box<dyn ServerAddressResolver>,
    redirect_handler: Box<dyn ServerRedirectHandler>,
    address_check: Box<dyn AddressCheck>,
}
impl Default for ServerNameResolver {
    fn default() -> Self {
        Self::new(
            Box::new(SystemServerAddressResolver),
            create_dns_srv_redirect_handler(),
            Box::new(AllowAllAddressCheck),
        )
    }
}
impl ServerNameResolver {
    pub fn new(
        resolver: Box<dyn ServerAddressResolver>,
        redirect_handler: Box<dyn ServerRedirectHandler>,
        address_check: Box<dyn AddressCheck>,
    ) -> Self {
        Self {
            resolver,
            redirect_handler,
            address_check,
        }
    }
    pub fn resolve_address(&self, address: &ServerAddress) -> Option<ResolvedServerAddress> {
        let resolved = self.resolver.resolve(address);
        let allowed = match &resolved {
            Some(res) => self.address_check.is_resolved_address_allowed(res),
            None => true,
        };
        if allowed && self.address_check.is_address_allowed(address) {
            let mut final_resolved = resolved;
            if let Some(redirected) = self.redirect_handler.lookup_redirect(address) {
                final_resolved = self
                    .resolver
                    .resolve(&redirected)
                    .filter(|res| self.address_check.is_resolved_address_allowed(res));
            }
            final_resolved
        } else {
            None
        }
    }
}
