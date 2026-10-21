use std::{collections::VecDeque, fmt};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedByteQueue {
    capacity: usize,
    bytes: VecDeque<u8>,
    closed: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ByteQueueError {
    ZeroCapacity,
    Closed,
    Backpressure {
        capacity: usize,
        queued: usize,
        incoming: usize,
    },
}
impl fmt::Display for ByteQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroCapacity => formatter.write_str("byte queue capacity must be non-zero"),
            Self::Closed => formatter.write_str("byte queue is closed"),
            Self::Backpressure {
                capacity,
                queued,
                incoming,
            } => {
                write!(
                    formatter,
                    "byte queue backpressure: {queued} queued + {incoming} incoming exceeds {capacity}"
                )
            }
        }
    }
}
impl std::error::Error for ByteQueueError {}
impl BoundedByteQueue {
    pub fn new(capacity: usize) -> Result<Self, ByteQueueError> {
        if capacity == 0 {
            return Err(ByteQueueError::ZeroCapacity);
        }
        Ok(Self {
            capacity,
            bytes: VecDeque::new(),
            closed: false,
        })
    }
    pub fn capacity(&self) -> usize {
        self.capacity
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    pub fn remaining_capacity(&self) -> usize {
        self.capacity - self.bytes.len()
    }
    pub fn push_frame(&mut self, frame: &[u8]) -> Result<(), ByteQueueError> {
        if self.closed {
            return Err(ByteQueueError::Closed);
        }
        if frame.len() > self.remaining_capacity() {
            return Err(ByteQueueError::Backpressure {
                capacity: self.capacity,
                queued: self.bytes.len(),
                incoming: frame.len(),
            });
        }
        self.bytes.extend(frame.iter().copied());
        Ok(())
    }
    pub fn read_into(&mut self, destination: &mut [u8]) -> usize {
        let count = destination.len().min(self.bytes.len());
        for byte in &mut destination[..count] {
            *byte = self.bytes.pop_front().expect("queue length was checked");
        }
        count
    }
    pub fn close(&mut self) {
        self.closed = true;
    }
    pub fn is_closed(&self) -> bool {
        self.closed
    }
    pub fn is_eof(&self) -> bool {
        self.closed && self.bytes.is_empty()
    }
}
