use crate::{
    AssetStore, HttpHeader, HttpUrl, SafeFileName, SecretBytes, SecretKey, StoreKey,
    StoreLocationId, VirtualPath,
};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
pub type PlatformFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;
pub type PlatformResult<T> = Result<T, PlatformError>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformErrorKind {
    Cancelled,
    Denied,
    Integrity,
    InvalidInput,
    Io,
    Network,
    NotFound,
    Unavailable,
    Unsupported,
    Other,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformError {
    pub kind: PlatformErrorKind,
    pub message: String,
}
impl PlatformError {
    pub fn new(kind: PlatformErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
    pub fn unsupported(operation: impl Into<String>) -> Self {
        Self::new(PlatformErrorKind::Unsupported, operation)
    }
}
impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}
impl std::error::Error for PlatformError {}
pub trait PersistentStore {
    fn get<'a>(&'a self, key: StoreKey) -> PlatformFuture<'a, PlatformResult<Option<Vec<u8>>>>;
    fn put<'a>(&'a self, key: StoreKey, value: Vec<u8>) -> PlatformFuture<'a, PlatformResult<()>>;
    fn delete<'a>(&'a self, key: StoreKey) -> PlatformFuture<'a, PlatformResult<()>>;
}
pub trait SecretStore {
    fn get<'a>(&'a self, key: SecretKey)
    -> PlatformFuture<'a, PlatformResult<Option<SecretBytes>>>;
    fn put<'a>(
        &'a self,
        key: SecretKey,
        value: SecretBytes,
    ) -> PlatformFuture<'a, PlatformResult<()>>;
    fn delete<'a>(&'a self, key: SecretKey) -> PlatformFuture<'a, PlatformResult<()>>;
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResourceSnapshotRequest {
    pub required_prefixes: Vec<VirtualPath>,
}
impl ResourceSnapshotRequest {
    pub fn all() -> Self {
        Self::default()
    }
    pub fn for_prefixes(required_prefixes: Vec<VirtualPath>) -> Self {
        Self { required_prefixes }
    }
    pub fn includes(&self, path: &VirtualPath) -> bool {
        self.required_prefixes.is_empty()
            || self
                .required_prefixes
                .iter()
                .any(|prefix| path.starts_with(prefix))
    }
}
pub trait ResourceSnapshot {
    fn read(&self, path: &VirtualPath) -> PlatformResult<Option<Vec<u8>>>;
    fn read_view(&self, path: &VirtualPath) -> PlatformResult<Option<crate::ResourceBytes>> {
        self.read_shared(path)
            .map(|bytes| bytes.map(crate::ResourceBytes::from_shared))
    }
    fn read_shared(&self, path: &VirtualPath) -> PlatformResult<Option<std::sync::Arc<Vec<u8>>>> {
        self.read(path).map(|bytes| bytes.map(std::sync::Arc::new))
    }
    fn list(&self, prefix: &VirtualPath) -> PlatformResult<Vec<VirtualPath>>;
}
pub trait ResourceStore {
    fn prepare_snapshot<'a>(
        &'a self,
        request: ResourceSnapshotRequest,
    ) -> PlatformFuture<'a, PlatformResult<Box<dyn ResourceSnapshot>>>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Head,
    Post,
    Put,
    Delete,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpByteRange {
    pub start: u64,
    pub end_inclusive: Option<u64>,
}
impl HttpByteRange {
    pub fn new(start: u64, end_inclusive: Option<u64>) -> PlatformResult<Self> {
        if let Some(end) = end_inclusive
            && end < start
        {
            return Err(PlatformError::new(
                PlatformErrorKind::InvalidInput,
                "HTTP byte range ends before it starts",
            ));
        }
        Ok(Self {
            start,
            end_inclusive,
        })
    }
}
#[derive(Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: HttpUrl,
    pub headers: Vec<HttpHeader>,
    pub body: Vec<u8>,
    pub byte_range: Option<HttpByteRange>,
}
impl HttpRequest {
    pub fn get(url: HttpUrl) -> Self {
        Self {
            method: HttpMethod::Get,
            url,
            headers: Vec::new(),
            body: Vec::new(),
            byte_range: None,
        }
    }
}
impl fmt::Debug for HttpRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpRequest")
            .field("method", &self.method)
            .field("url", &self.url)
            .field("headers", &self.headers)
            .field("body_len", &self.body.len())
            .field("byte_range", &self.byte_range)
            .finish()
    }
}
pub trait HttpResponse {
    fn status(&self) -> u16;
    fn headers(&self) -> &[HttpHeader];
    fn next_chunk<'a>(&'a mut self) -> PlatformFuture<'a, PlatformResult<Option<Vec<u8>>>>;
    fn is_success(&self) -> bool {
        (200..300).contains(&self.status())
    }
}
pub trait HttpClient {
    fn request<'a>(
        &'a self,
        request: HttpRequest,
    ) -> PlatformFuture<'a, PlatformResult<Box<dyn HttpResponse>>>;
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkTarget {
    host: String,
    port: u16,
}
impl NetworkTarget {
    pub fn new(host: impl Into<String>, port: u16) -> PlatformResult<Self> {
        let host = host.into();
        if host.is_empty()
            || host.len() > 255
            || host
                .chars()
                .any(|character| character.is_control() || character.is_whitespace())
        {
            return Err(PlatformError::new(
                PlatformErrorKind::InvalidInput,
                "network target host is invalid",
            ));
        }
        if port == 0 {
            return Err(PlatformError::new(
                PlatformErrorKind::InvalidInput,
                "network target port must be non-zero",
            ));
        }
        Ok(Self { host, port })
    }
    pub fn host(&self) -> &str {
        &self.host
    }
    pub fn port(&self) -> u16 {
        self.port
    }
}
pub trait TransportReader {
    fn read<'a>(
        &'a mut self,
        destination: &'a mut [u8],
    ) -> PlatformFuture<'a, PlatformResult<usize>>;
}
pub trait TransportWriter {
    fn write_all<'a>(&'a mut self, source: &'a [u8]) -> PlatformFuture<'a, PlatformResult<()>>;
    fn close<'a>(&'a mut self) -> PlatformFuture<'a, PlatformResult<()>>;
}
pub trait TransportStream {
    fn into_split(self: Box<Self>) -> (Box<dyn TransportReader>, Box<dyn TransportWriter>);
}
#[cfg(not(target_arch = "wasm32"))]
pub trait NativeTransportStream: Send {
    fn into_native_split(
        self: Box<Self>,
    ) -> (
        Box<dyn tokio::io::AsyncRead + Send + Unpin>,
        Box<dyn tokio::io::AsyncWrite + Send + Unpin>,
    );
}
pub trait TransportFactory {
    fn connect<'a>(
        &'a self,
        target: NetworkTarget,
    ) -> PlatformFuture<'a, PlatformResult<Box<dyn TransportStream>>>;
}
#[cfg(not(target_arch = "wasm32"))]
pub trait NativeTransportFactory {
    fn connect_native<'a>(
        &'a self,
        target: NetworkTarget,
    ) -> PlatformFuture<'a, PlatformResult<Box<dyn NativeTransportStream>>>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioState {
    Running,
    Suspended,
}
pub trait AudioBackend {
    fn state(&self) -> AudioState;
    fn suspend<'a>(&'a self) -> PlatformFuture<'a, PlatformResult<()>>;
    fn resume<'a>(&'a self) -> PlatformFuture<'a, PlatformResult<()>>;
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImportRequest {
    pub accepted_mime_types: Vec<String>,
    pub accepted_extensions: Vec<String>,
    pub allow_multiple: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedFile {
    pub display_name: SafeFileName,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportFile {
    pub suggested_name: SafeFileName,
    pub mime_type: String,
    pub bytes: Vec<u8>,
}
pub trait PlatformActions {
    fn open_external_url<'a>(&'a self, url: HttpUrl) -> PlatformFuture<'a, PlatformResult<()>>;
    fn read_clipboard<'a>(&'a self) -> PlatformFuture<'a, PlatformResult<Option<String>>>;
    fn write_clipboard<'a>(&'a self, contents: String) -> PlatformFuture<'a, PlatformResult<()>>;
    fn import_files<'a>(
        &'a self,
        request: ImportRequest,
    ) -> PlatformFuture<'a, PlatformResult<Vec<ImportedFile>>>;
    fn export_file<'a>(&'a self, file: ExportFile) -> PlatformFuture<'a, PlatformResult<()>>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskClass {
    Local,
    Background,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskHandle(pub u64);
pub trait TaskExecutor {
    fn spawn<'a>(
        &'a self,
        class: TaskClass,
        label: String,
        task: PlatformFuture<'static, ()>,
    ) -> PlatformFuture<'a, PlatformResult<TaskHandle>>;
    fn cancel<'a>(&'a self, task: TaskHandle) -> PlatformFuture<'a, PlatformResult<()>>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformLocation {
    Data,
    Cache,
    ResourcePacks,
    Logs,
}
pub trait PlatformPaths {
    fn location<'a>(
        &'a self,
        location: PlatformLocation,
    ) -> PlatformFuture<'a, PlatformResult<StoreLocationId>>;
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformKind {
    Desktop,
    Android,
    Web,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformCapabilities {
    pub kind: PlatformKind,
    pub supports_external_processes: bool,
    pub supports_raw_tcp: bool,
    pub supports_file_paths: bool,
    pub supports_pointer_lock: bool,
    pub supports_background_workers: bool,
}
impl PlatformCapabilities {
    pub const fn desktop() -> Self {
        Self {
            kind: PlatformKind::Desktop,
            supports_external_processes: true,
            supports_raw_tcp: true,
            supports_file_paths: true,
            supports_pointer_lock: false,
            supports_background_workers: true,
        }
    }
    pub const fn android() -> Self {
        Self {
            kind: PlatformKind::Android,
            supports_external_processes: false,
            supports_raw_tcp: true,
            supports_file_paths: false,
            supports_pointer_lock: false,
            supports_background_workers: true,
        }
    }
    pub const fn web() -> Self {
        Self {
            kind: PlatformKind::Web,
            supports_external_processes: false,
            supports_raw_tcp: false,
            supports_file_paths: false,
            supports_pointer_lock: true,
            supports_background_workers: true,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceSize {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}
impl SurfaceSize {
    pub const fn new(width: u32, height: u32, scale_factor: f64) -> Self {
        Self {
            width,
            height,
            scale_factor,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SurfaceLifecycleEvent {
    Resumed,
    Suspended,
    SurfaceAvailable(SurfaceSize),
    SurfaceResized(SurfaceSize),
    SurfaceLost,
    LowMemory,
}
pub struct PlatformServices<'a> {
    pub persistent: &'a dyn PersistentStore,
    pub secrets: &'a dyn SecretStore,
    pub assets: &'a dyn AssetStore,
    pub resources: &'a dyn ResourceStore,
    pub http: &'a dyn HttpClient,
    pub transport: &'a dyn TransportFactory,
    pub audio: &'a dyn AudioBackend,
    pub actions: &'a dyn PlatformActions,
    pub tasks: &'a dyn TaskExecutor,
    pub paths: &'a dyn PlatformPaths,
    pub capabilities: PlatformCapabilities,
}
