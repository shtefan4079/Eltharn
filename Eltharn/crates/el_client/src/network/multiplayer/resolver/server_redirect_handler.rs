#![allow(dead_code)]
use crate::network::multiplayer::resolver::server_address::{DEFAULT_PORT, ServerAddress};
pub trait ServerRedirectHandler {
    fn lookup_redirect(&self, address: &ServerAddress) -> Option<ServerAddress>;
}
pub struct EmptyServerRedirectHandler;
impl ServerRedirectHandler for EmptyServerRedirectHandler {
    fn lookup_redirect(&self, _address: &ServerAddress) -> Option<ServerAddress> {
        None
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub struct DnsSrvServerRedirectHandler {
    runtime: tokio::runtime::Runtime,
    resolver: hickory_resolver::TokioResolver,
}
#[cfg(not(target_arch = "wasm32"))]
impl DnsSrvServerRedirectHandler {
    fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let resolver = {
            let _entered = runtime.enter();
            let mut builder = hickory_resolver::TokioResolver::builder_tokio()?;
            builder.options_mut().attempts = 1;
            builder.build()
        };
        Ok(Self { runtime, resolver })
    }
}
pub fn create_dns_srv_redirect_handler() -> Box<dyn ServerRedirectHandler> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match DnsSrvServerRedirectHandler::new() {
            Ok(handler) => return Box::new(handler),
            Err(error) => {
                log::error!("Failed to initialize DNS SRV redirect resolver: {error}")
            }
        }
    }
    Box::new(EmptyServerRedirectHandler)
}
#[cfg(not(target_arch = "wasm32"))]
impl ServerRedirectHandler for DnsSrvServerRedirectHandler {
    fn lookup_redirect(&self, address: &ServerAddress) -> Option<ServerAddress> {
        lookup_srv_redirect_like_java(address, |query| {
            self.runtime.block_on(async {
                let answers = self.resolver.srv_lookup(query).await.ok()?;
                first_srv_redirect_like_java(&answers)
            })
        })
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn first_srv_redirect_like_java(
    answers: &hickory_resolver::lookup::SrvLookup,
) -> Option<ServerAddress> {
    let first = answers.iter().next()?;
    Some(ServerAddress::new(first.target().to_utf8(), first.port()))
}
fn lookup_srv_redirect_like_java(
    address: &ServerAddress,
    lookup: impl FnOnce(&str) -> Option<ServerAddress>,
) -> Option<ServerAddress> {
    if address.port() != DEFAULT_PORT {
        return None;
    }
    lookup(&format!("_minecraft._tcp.{}", address.host()))
}
