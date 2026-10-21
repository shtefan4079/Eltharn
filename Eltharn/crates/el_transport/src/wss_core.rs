use crate::{BoundedByteQueue, ByteQueueError};
use el_platform_api::{
    PlatformError, PlatformErrorKind, PlatformFuture, PlatformResult, TransportReader,
    TransportStream, TransportWriter,
};
use std::{
    cell::RefCell,
    future::poll_fn,
    rc::Rc,
    task::{Poll, Waker},
};
pub struct LocalWssByteStream {
    shared: Rc<RefCell<LocalWssState>>,
}
#[derive(Clone)]
pub struct LocalWssDriver {
    shared: Rc<RefCell<LocalWssState>>,
}
struct LocalWssState {
    inbound: BoundedByteQueue,
    outbound: BoundedByteQueue,
    inbound_reader_waker: Option<Waker>,
    outbound_writer_waker: Option<Waker>,
}
impl LocalWssByteStream {
    pub fn new(
        inbound_capacity: usize,
        outbound_capacity: usize,
    ) -> Result<(Self, LocalWssDriver), ByteQueueError> {
        let shared = Rc::new(RefCell::new(LocalWssState {
            inbound: BoundedByteQueue::new(inbound_capacity)?,
            outbound: BoundedByteQueue::new(outbound_capacity)?,
            inbound_reader_waker: None,
            outbound_writer_waker: None,
        }));
        Ok((
            Self {
                shared: Rc::clone(&shared),
            },
            LocalWssDriver { shared },
        ))
    }
}
impl LocalWssDriver {
    pub fn receive_binary_frame(&self, frame: &[u8]) -> Result<(), ByteQueueError> {
        let wake_reader = {
            let mut state = self.shared.borrow_mut();
            state.inbound.push_frame(frame)?;
            if frame.is_empty() {
                None
            } else {
                state.inbound_reader_waker.take()
            }
        };
        wake(wake_reader);
        Ok(())
    }
    pub fn close_remote(&self) {
        let wake_reader = {
            let mut state = self.shared.borrow_mut();
            state.inbound.close();
            state.inbound_reader_waker.take()
        };
        wake(wake_reader);
    }
    pub fn take_outbound_chunk(&self, max_bytes: usize) -> Option<Vec<u8>> {
        if max_bytes == 0 {
            return None;
        }
        let (bytes, wake_writer) = {
            let mut state = self.shared.borrow_mut();
            let count = max_bytes.min(state.outbound.len());
            if count == 0 {
                return None;
            }
            let mut bytes = vec![0; count];
            let drained = state.outbound.read_into(&mut bytes);
            debug_assert_eq!(drained, count);
            (bytes, state.outbound_writer_waker.take())
        };
        wake(wake_writer);
        Some(bytes)
    }
    pub fn is_local_write_drained(&self) -> bool {
        self.shared.borrow().outbound.is_eof()
    }
    pub fn outbound_len(&self) -> usize {
        self.shared.borrow().outbound.len()
    }
    pub fn inbound_len(&self) -> usize {
        self.shared.borrow().inbound.len()
    }
}
struct LocalWssReader {
    shared: Rc<RefCell<LocalWssState>>,
}
struct LocalWssWriter {
    shared: Rc<RefCell<LocalWssState>>,
}
fn wake(waker: Option<Waker>) {
    if let Some(waker) = waker {
        waker.wake();
    }
}
fn closed_writer_error() -> PlatformError {
    PlatformError::new(
        PlatformErrorKind::Network,
        "local WSS transport write half is closed",
    )
}
fn oversized_write_error(source_len: usize, capacity: usize) -> PlatformError {
    PlatformError::new(
        PlatformErrorKind::InvalidInput,
        format!(
            "local WSS transport write of {source_len} bytes exceeds its {capacity}-byte outbound capacity"
        ),
    )
}
fn unexpected_queue_error(operation: &'static str, error: ByteQueueError) -> PlatformError {
    let kind = match error {
        ByteQueueError::Backpressure { .. } => PlatformErrorKind::Unavailable,
        ByteQueueError::Closed => PlatformErrorKind::Network,
        ByteQueueError::ZeroCapacity => PlatformErrorKind::InvalidInput,
    };
    PlatformError::new(kind, format!("local WSS transport {operation}: {error}"))
}
impl TransportReader for LocalWssReader {
    fn read<'a>(
        &'a mut self,
        destination: &'a mut [u8],
    ) -> PlatformFuture<'a, PlatformResult<usize>> {
        Box::pin(poll_fn(move |context| {
            if destination.is_empty() {
                return Poll::Ready(Ok(0));
            }
            let mut state = self.shared.borrow_mut();
            let count = state.inbound.read_into(destination);
            if count != 0 || state.inbound.is_eof() {
                return Poll::Ready(Ok(count));
            }
            state.inbound_reader_waker = Some(context.waker().clone());
            Poll::Pending
        }))
    }
}
impl TransportWriter for LocalWssWriter {
    fn write_all<'a>(&'a mut self, source: &'a [u8]) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(poll_fn(move |context| {
            let mut state = self.shared.borrow_mut();
            if state.outbound.is_closed() {
                return Poll::Ready(Err(closed_writer_error()));
            }
            if source.len() > state.outbound.capacity() {
                return Poll::Ready(Err(oversized_write_error(
                    source.len(),
                    state.outbound.capacity(),
                )));
            }
            if source.len() <= state.outbound.remaining_capacity() {
                return Poll::Ready(
                    state
                        .outbound
                        .push_frame(source)
                        .map_err(|error| unexpected_queue_error("write", error)),
                );
            }
            state.outbound_writer_waker = Some(context.waker().clone());
            Poll::Pending
        }))
    }
    fn close<'a>(&'a mut self) -> PlatformFuture<'a, PlatformResult<()>> {
        Box::pin(async move {
            self.shared.borrow_mut().outbound.close();
            Ok(())
        })
    }
}
impl TransportStream for LocalWssByteStream {
    fn into_split(self: Box<Self>) -> (Box<dyn TransportReader>, Box<dyn TransportWriter>) {
        let Self { shared } = *self;
        (
            Box::new(LocalWssReader {
                shared: Rc::clone(&shared),
            }),
            Box::new(LocalWssWriter { shared }),
        )
    }
}
