#![forbid(unsafe_code)]

mod compatibility;
mod connection;
mod identifiers;
mod registry;
pub mod shared;
mod wire;

pub use compatibility::{
    ELTHARN_PROTOCOL_API_VERSION, ProtocolApiVersion, ProtocolDescriptor, ProtocolDescriptorError,
    ProtocolPreflightFailure, ProtocolPreflightRequest, ProtocolPreflightResult,
    ProtocolPreflightSuccess, preflight,
};
#[cfg(not(target_arch = "wasm32"))]
pub use connection::NativeInjectedTransport;
pub use connection::{
    ClientConnectRequest, LocalInjectedTransport, ProtocolClientInformation,
    ProtocolConnectRequest, ProtocolConnection, ProtocolConnectionContext, ProtocolConnectionError,
    ProtocolConnectionErrorKind, ProtocolDisconnectRequest, ProtocolFactory, ProtocolFuture,
    ProtocolPlayerIdentity, ProtocolServerAddress,
};
pub use el_platform_api::NetworkTarget;
pub use identifiers::{LogicalIdError, ProtocolImplementationId};
pub use registry::{ProtocolRegistry, ProtocolRegistryError};
pub use shared::{
    AccountId, AccountIdError, AssetCompatibilityId, AssetManifestHash, AssetSelection,
    AssetStoreId, GameLaunchConfig, GameVersionId, LaunchId, ProtocolAbiVersion, ProtocolId,
    ProtocolIdError, ServerTarget, VersionCompatibilityBinding,
};
pub use uuid::Uuid;
pub use wire::{
    ClientNetworkEvent, ClientOutboundPacket, KnownInboundEventKind, KnownOutboundActionKind,
    ProtocolContractItem, ProtocolInboundEvent, ProtocolLogLevel, ProtocolOutboundAction,
    ProtocolPayload,
};
