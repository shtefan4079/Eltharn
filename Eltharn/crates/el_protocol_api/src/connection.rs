use crate::{
    ProtocolDescriptor, ProtocolInboundEvent, ProtocolOutboundAction, ProtocolPreflightRequest,
    ProtocolPreflightResult, ProtocolPreflightSuccess, preflight,
};
#[cfg(not(target_arch = "wasm32"))]
use el_platform_api::NativeTransportStream;
use el_platform_api::{NetworkTarget, TransportStream};
use std::{collections::BTreeMap, fmt, future::Future, pin::Pin};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClientConnectRequest {
    pub server: String,
    pub username: String,
    pub language: String,
    pub uuid: Uuid,
    pub view_distance: u8,
    pub main_hand_left: bool,
    pub allows_listing: bool,
    pub server_pack_status: u8,
    pub model_customisation: u8,
    pub server_cookies: std::collections::HashMap<String, Vec<u8>>,
}
impl ClientConnectRequest {
    pub fn offline(server: impl Into<String>, username: impl Into<String>) -> Self {
        let username = username.into();
        let digest = md5::compute(format!("OfflinePlayer:{username}").as_bytes());
        Self {
            server: server.into(),
            uuid: uuid::Builder::from_md5_bytes(digest.0).into_uuid(),
            username,
            language: "en_us".to_owned(),
            view_distance: 12,
            main_hand_left: false,
            allows_listing: true,
            server_pack_status: 0,
            model_customisation: 0x7F,
            server_cookies: std::collections::HashMap::new(),
        }
    }
}
pub type ProtocolFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolServerAddress(String);
impl ProtocolServerAddress {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolConnectionError> {
        let value = value.into();
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(ProtocolConnectionError::invalid_input(
                "server address is empty or contains control characters",
            ));
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
    pub fn into_inner(self) -> String {
        self.0
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolPlayerIdentity {
    pub username: String,
    pub uuid: Uuid,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtocolClientInformation {
    pub view_distance: u8,
    pub main_hand_left: bool,
    pub allows_listing: bool,
    pub server_pack_status: u8,
    pub model_customisation: u8,
}
impl Default for ProtocolClientInformation {
    fn default() -> Self {
        Self {
            view_distance: 12,
            main_hand_left: false,
            allows_listing: true,
            server_pack_status: 0,
            model_customisation: 0x7F,
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct ProtocolConnectRequest {
    preflight: ProtocolPreflightSuccess,
    pub server: ProtocolServerAddress,
    pub player: ProtocolPlayerIdentity,
    pub client_information: ProtocolClientInformation,
    pub transfer_cookies: BTreeMap<String, Vec<u8>>,
}
impl ProtocolConnectRequest {
    pub fn new(
        preflight: ProtocolPreflightSuccess,
        server: ProtocolServerAddress,
        player: ProtocolPlayerIdentity,
        client_information: ProtocolClientInformation,
        transfer_cookies: BTreeMap<String, Vec<u8>>,
    ) -> Self {
        Self {
            preflight,
            server,
            player,
            client_information,
            transfer_cookies,
        }
    }
    pub fn preflight(&self) -> &ProtocolPreflightSuccess {
        &self.preflight
    }
}
impl fmt::Debug for ProtocolConnectRequest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ProtocolConnectRequest")
            .field("protocol", &self.preflight.descriptor().protocol_id)
            .field("server", &self.server)
            .field("player", &self.player)
            .field("client_information", &self.client_information)
            .field(
                "transfer_cookie_keys",
                &self.transfer_cookies.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}
pub struct LocalInjectedTransport {
    handshake_target: NetworkTarget,
    stream: Box<dyn TransportStream>,
}
impl LocalInjectedTransport {
    pub fn new(handshake_target: NetworkTarget, stream: Box<dyn TransportStream>) -> Self {
        Self {
            handshake_target,
            stream,
        }
    }
    pub fn handshake_target(&self) -> &NetworkTarget {
        &self.handshake_target
    }
    pub fn into_parts(self) -> (NetworkTarget, Box<dyn TransportStream>) {
        (self.handshake_target, self.stream)
    }
}
impl fmt::Debug for LocalInjectedTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LocalInjectedTransport")
            .field("transport", &"local_injected")
            .finish()
    }
}
#[cfg(not(target_arch = "wasm32"))]
pub struct NativeInjectedTransport {
    handshake_target: NetworkTarget,
    stream: Box<dyn NativeTransportStream>,
}
#[cfg(not(target_arch = "wasm32"))]
impl NativeInjectedTransport {
    pub fn new(handshake_target: NetworkTarget, stream: Box<dyn NativeTransportStream>) -> Self {
        Self {
            handshake_target,
            stream,
        }
    }
    pub fn handshake_target(&self) -> &NetworkTarget {
        &self.handshake_target
    }
    pub fn into_parts(self) -> (NetworkTarget, Box<dyn NativeTransportStream>) {
        (self.handshake_target, self.stream)
    }
}
#[cfg(not(target_arch = "wasm32"))]
impl fmt::Debug for NativeInjectedTransport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeInjectedTransport")
            .field("transport", &"native_injected")
            .finish()
    }
}
enum ProtocolTransportContext {
    LegacyFactoryOwned,
    LocalInjected(LocalInjectedTransport),
    #[cfg(not(target_arch = "wasm32"))]
    NativeInjected(NativeInjectedTransport),
}
pub struct ProtocolConnectionContext {
    transport: ProtocolTransportContext,
}
impl ProtocolConnectionContext {
    pub fn injected(handshake_target: NetworkTarget, stream: Box<dyn TransportStream>) -> Self {
        Self {
            transport: ProtocolTransportContext::LocalInjected(LocalInjectedTransport::new(
                handshake_target,
                stream,
            )),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn injected_native(
        handshake_target: NetworkTarget,
        stream: Box<dyn NativeTransportStream>,
    ) -> Self {
        Self {
            transport: ProtocolTransportContext::NativeInjected(NativeInjectedTransport::new(
                handshake_target,
                stream,
            )),
        }
    }
    pub fn legacy_factory_owned() -> Self {
        Self {
            transport: ProtocolTransportContext::LegacyFactoryOwned,
        }
    }
    pub fn has_injected_transport(&self) -> bool {
        !matches!(self.transport, ProtocolTransportContext::LegacyFactoryOwned)
    }
    pub fn handshake_target(&self) -> Option<&NetworkTarget> {
        match &self.transport {
            ProtocolTransportContext::LegacyFactoryOwned => None,
            ProtocolTransportContext::LocalInjected(transport) => {
                Some(transport.handshake_target())
            }
            #[cfg(not(target_arch = "wasm32"))]
            ProtocolTransportContext::NativeInjected(transport) => {
                Some(transport.handshake_target())
            }
        }
    }
    pub fn has_native_transport(&self) -> bool {
        #[cfg(not(target_arch = "wasm32"))]
        {
            matches!(self.transport, ProtocolTransportContext::NativeInjected(_))
        }
        #[cfg(target_arch = "wasm32")]
        {
            false
        }
    }
    pub fn into_local_injected_transport(self) -> Option<LocalInjectedTransport> {
        match self.transport {
            ProtocolTransportContext::LocalInjected(transport) => Some(transport),
            ProtocolTransportContext::LegacyFactoryOwned => None,
            #[cfg(not(target_arch = "wasm32"))]
            ProtocolTransportContext::NativeInjected(_) => None,
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn into_native_injected_transport(self) -> Option<NativeInjectedTransport> {
        match self.transport {
            ProtocolTransportContext::NativeInjected(transport) => Some(transport),
            ProtocolTransportContext::LegacyFactoryOwned
            | ProtocolTransportContext::LocalInjected(_) => None,
        }
    }
}
impl fmt::Debug for ProtocolConnectionContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let transport = match &self.transport {
            ProtocolTransportContext::LegacyFactoryOwned => "legacy_factory_owned",
            ProtocolTransportContext::LocalInjected(_) => "local_injected",
            #[cfg(not(target_arch = "wasm32"))]
            ProtocolTransportContext::NativeInjected(_) => "native_injected",
        };
        formatter
            .debug_struct("ProtocolConnectionContext")
            .field("transport", &transport)
            .finish()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolDisconnectRequest {
    pub reason: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolConnectionError {
    pub kind: ProtocolConnectionErrorKind,
    pub message: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolConnectionErrorKind {
    Cancelled,
    Compatibility,
    Disconnected,
    InvalidInput,
    Protocol,
    Transport,
    Unsupported,
    Other,
}
impl ProtocolConnectionError {
    pub fn new(kind: ProtocolConnectionErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ProtocolConnectionErrorKind::InvalidInput, message)
    }
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ProtocolConnectionErrorKind::Unsupported, message)
    }
}
impl fmt::Display for ProtocolConnectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}
impl std::error::Error for ProtocolConnectionError {}
pub trait ProtocolFactory {
    fn descriptor(&self) -> ProtocolDescriptor;
    fn preflight(&self, request: ProtocolPreflightRequest) -> ProtocolPreflightResult {
        let descriptor = self.descriptor();
        preflight(&descriptor, request)
    }
    fn connect<'a>(
        &'a self,
        request: ProtocolConnectRequest,
        context: ProtocolConnectionContext,
    ) -> ProtocolFuture<'a, Result<Box<dyn ProtocolConnection>, ProtocolConnectionError>> {
        let descriptor = self.descriptor();
        if request.preflight().descriptor() != &descriptor {
            return Box::pin(async {
                Err(ProtocolConnectionError::new(
                    ProtocolConnectionErrorKind::Compatibility,
                    "connection preflight proof does not belong to this protocol factory",
                ))
            });
        }
        self.connect_preflighted(request, context)
    }
    fn connect_preflighted<'a>(
        &'a self,
        request: ProtocolConnectRequest,
        context: ProtocolConnectionContext,
    ) -> ProtocolFuture<'a, Result<Box<dyn ProtocolConnection>, ProtocolConnectionError>>;
}
pub trait ProtocolConnection {
    fn descriptor(&self) -> &ProtocolDescriptor;
    fn next_event<'a>(
        &'a mut self,
    ) -> ProtocolFuture<'a, Result<Option<ProtocolInboundEvent>, ProtocolConnectionError>>;
    fn send<'a>(
        &'a mut self,
        action: ProtocolOutboundAction,
    ) -> ProtocolFuture<'a, Result<(), ProtocolConnectionError>>;
    fn disconnect<'a>(
        &'a mut self,
        request: ProtocolDisconnectRequest,
    ) -> ProtocolFuture<'a, Result<(), ProtocolConnectionError>>;
}
