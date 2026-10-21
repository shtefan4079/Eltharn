use crate::{BoundedByteQueue, ByteQueueError};
use el_platform_api::{
    PlatformError, PlatformErrorKind, PlatformFuture, PlatformResult, TransportReader,
    TransportStream, TransportWriter,
};
use std::sync::{Arc, Mutex};
pub struct MemoryTransport {
    incoming: Arc<Mutex<BoundedByteQueue>>,
    outgoing: Arc<Mutex<BoundedByteQueue>>,
}
pub fn memory_transport_pair(
    capacity: usize,
) -> Result<(MemoryTransport, MemoryTransport), ByteQueueError> {
    let left_incoming = Arc::new(Mutex::new(BoundedByteQueue::new(capacity)?));
    let right_incoming = Arc::new(Mutex::new(BoundedByteQueue::new(capacity)?));
    Ok((
        MemoryTransport {
            incoming: Arc::clone(&left_incoming),
            outgoing: Arc::clone(&right_incoming),
        },
        MemoryTransport {
            incoming: right_incoming,
            outgoing: left_incoming,
        },
    ))
}
struct MemoryReader {
    incoming: Arc<Mutex<BoundedByteQueue>>,
}
struct MemoryWriter {
    outgoing: Arc<Mutex<BoundedByteQueue>>,
}
fn lock_error(operation: &'static str) -> PlatformError {
    PlatformError::new(
        PlatformErrorKind::Other,
        format!("in-memory transport {operation} lock was poisoned"),
    )
}
fn queue_error(operation: &'static str, error: ByteQueueError) -> PlatformError {
    let kind = match error {
        ByteQueueError::Backpressure { .. } => PlatformErrorKind::Unavailable,
        ByteQueueError::Closed => PlatformErrorKind::Network,
        ByteQueueError::ZeroCapacity => PlatformErrorKind::InvalidInput,
    };
    PlatformError::new(kind, format!("in-memory transport {operation}: {error}"))
}
impl TransportReader for MemoryReader {
    fn read<'a>(
        &'a mut self,
        destination: &'a mut [u8],
    ) -> PlatformFuture<'a, PlatformResult<usize>> {
        Box::pin(async move {
            if destination.is_empty() {
                return Ok(0);
            }
            let mut incoming = self.incoming.lock().map_err(|_| lock_error("read"))?;
            let count = incoming.read_into(destination);
            if count != 0 || incoming.is_eof() {
                return Ok(count);
            }
            Err(PlatformError::new(
                PlatformErrorKind::Unavailable,
                "in-memory transport read would block",
            ))
        })
    }
}
impl TransportWriter for MemoryWriter {
    fn write_all<'a>(&'a mut self, source: &'a [u8]) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(async move {
            let mut outgoing = self.outgoing.lock().map_err(|_| lock_error("write"))?;
            outgoing
                .push_frame(source)
                .map_err(|error| queue_error("write", error))
        })
    }
    fn close<'a>(&'a mut self) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(async move {
            let mut outgoing = self.outgoing.lock().map_err(|_| lock_error("close"))?;
            outgoing.close();
            Ok(())
        })
    }
}
impl TransportStream for MemoryTransport {
    fn into_split(self: Box<Self>) -> (Box<dyn TransportReader>, Box<dyn TransportWriter>) {
        let Self { incoming, outgoing } = *self;
        (
            Box::new(MemoryReader { incoming }),
            Box::new(MemoryWriter { outgoing }),
        )
    }
}
