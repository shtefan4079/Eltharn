#![forbid(unsafe_code)]

mod asset_store;
mod identifier;
mod memory;
mod resource_bytes;
mod services;

pub use resource_bytes::ResourceBytes;

pub use asset_store::{
    AssetNamespace, AssetObjectId, AssetReleaseCommit, AssetReleaseEntry, AssetReleaseId,
    AssetReleaseSnapshot, AssetReleaseSnapshotInfo, AssetReleaseSnapshotKey, AssetStagingId,
    AssetStagingOpen, AssetStagingRequest, AssetStagingState, AssetStore, ReleaseSnapshotAdapter,
    UserPackNamespaceId, VerifiedAssetObject,
};
pub use identifier::{
    HttpHeader, HttpUrl, IdentifierError, SafeFileName, SecretBytes, SecretKey, StoreKey,
    StoreLocationId, VirtualPath,
};
pub use memory::{MemoryAssetStore, MemoryPersistentStore, MemoryResourceStore, MemorySecretStore};
pub use services::{
    AudioBackend, AudioState, ExportFile, HttpByteRange, HttpClient, HttpMethod, HttpRequest,
    HttpResponse, ImportRequest, ImportedFile, NetworkTarget, PersistentStore, PlatformActions,
    PlatformCapabilities, PlatformError, PlatformErrorKind, PlatformFuture, PlatformKind,
    PlatformLocation, PlatformPaths, PlatformResult, PlatformServices, ResourceSnapshot,
    ResourceSnapshotRequest, ResourceStore, SecretStore, SurfaceLifecycleEvent, SurfaceSize,
    TaskClass, TaskExecutor, TaskHandle, TransportFactory, TransportReader, TransportStream,
    TransportWriter,
};

#[cfg(not(target_arch = "wasm32"))]
pub use services::{NativeTransportFactory, NativeTransportStream};
