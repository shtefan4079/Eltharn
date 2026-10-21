#![forbid(unsafe_code)]

mod memory;
mod queue;
mod wss_core;

#[cfg(not(target_arch = "wasm32"))]
mod native;

pub use memory::{MemoryTransport, memory_transport_pair};
#[cfg(not(target_arch = "wasm32"))]
pub use native::{TokioTcpTransport, TokioTcpTransportFactory};
pub use queue::{BoundedByteQueue, ByteQueueError};
pub use wss_core::{LocalWssByteStream, LocalWssDriver};
