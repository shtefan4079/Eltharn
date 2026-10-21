pub const CHANNEL: &str = "rustore:mods_v1";
pub const MAX_MESSAGE_BYTES: usize = 524_288;
pub const PAYLOAD_PER_FRAME: usize = 24_000;
pub const MAX_FRAMES: u16 = 22;
pub const HEADER_LEN: usize = 16;
pub const MAX_FRAME_BYTES: usize = 24_016;
pub const MAX_MESSAGE_ID: u64 = 9_007_199_254_740_991;
pub const MAGIC: [u8; 4] = *b"RM01";
pub fn fragment(id: u64, bytes: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    if id == 0 || id > MAX_MESSAGE_ID {
        return Err(format!(
            "invalid message id {id}: must be 1..={MAX_MESSAGE_ID}"
        ));
    }
    if bytes.is_empty() {
        return Err("empty message: nothing to fragment".to_string());
    }
    if bytes.len() > MAX_MESSAGE_BYTES {
        return Err(format!(
            "message too large: {} > {MAX_MESSAGE_BYTES}",
            bytes.len()
        ));
    }
    let count_usize = bytes.len().div_ceil(PAYLOAD_PER_FRAME);
    if count_usize == 0 || count_usize > MAX_FRAMES as usize {
        return Err(format!(
            "frame count {count_usize} out of range 1..={MAX_FRAMES}"
        ));
    }
    let count = count_usize as u16;
    let mut out = Vec::with_capacity(count_usize);
    for index_usize in 0..count_usize {
        let start = index_usize * PAYLOAD_PER_FRAME;
        let end = (start + PAYLOAD_PER_FRAME).min(bytes.len());
        let payload = &bytes[start..end];
        debug_assert!(!payload.is_empty() && payload.len() <= PAYLOAD_PER_FRAME);
        let mut frame = Vec::with_capacity(HEADER_LEN + payload.len());
        frame.extend_from_slice(&MAGIC);
        frame.extend_from_slice(&id.to_be_bytes());
        frame.extend_from_slice(&(index_usize as u16).to_be_bytes());
        frame.extend_from_slice(&count.to_be_bytes());
        frame.extend_from_slice(payload);
        debug_assert!(frame.len() <= MAX_FRAME_BYTES);
        out.push(frame);
    }
    Ok(out)
}
#[derive(Debug, Default)]
pub struct Reassembler {
    active_id: Option<u64>,
    active_count: u16,
    next_index: u16,
    buf: Vec<u8>,
    max_started: u64,
}
impl Reassembler {
    pub fn clear(&mut self) {
        self.reset_active();
    }
    pub fn last_accepted_id(&self) -> u64 {
        self.max_started
    }
    pub fn push(&mut self, bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
        if bytes.len() < HEADER_LEN {
            self.reset_active();
            return Err(format!("frame too short: {} < {HEADER_LEN}", bytes.len()));
        }
        if bytes.len() > MAX_FRAME_BYTES {
            self.reset_active();
            return Err(format!(
                "frame too large: {} > {MAX_FRAME_BYTES}",
                bytes.len()
            ));
        }
        if bytes[0..4] != MAGIC {
            self.reset_active();
            return Err("bad magic: expected R M 0 1".to_string());
        }
        let mut id_b = [0u8; 8];
        id_b.copy_from_slice(&bytes[4..12]);
        let id = u64::from_be_bytes(id_b);
        let index = u16::from_be_bytes([bytes[12], bytes[13]]);
        let count = u16::from_be_bytes([bytes[14], bytes[15]]);
        let payload = &bytes[HEADER_LEN..];
        if id == 0 || id > MAX_MESSAGE_ID {
            self.reset_active();
            return Err(format!("invalid message id {id}"));
        }
        if count == 0 || count > MAX_FRAMES {
            self.reset_active();
            return Err(format!(
                "invalid frame count {count}: must be 1..={MAX_FRAMES}"
            ));
        }
        if payload.is_empty() || payload.len() > PAYLOAD_PER_FRAME {
            self.reset_active();
            return Err(format!("invalid payload len {}", payload.len()));
        }
        if index >= count {
            self.reset_active();
            return Err(format!("invalid index {index} for count {count}"));
        }
        if (count as usize - 1).saturating_mul(PAYLOAD_PER_FRAME) >= MAX_MESSAGE_BYTES {
            self.reset_active();
            return Err(format!("declared count {count} exceeds message cap"));
        }
        match self.active_id {
            None => {
                if id <= self.max_started {
                    return Err(format!(
                        "replay message id {id}: must exceed {}",
                        self.max_started
                    ));
                }
                if index != 0 {
                    return Err(format!("expected index 0 for new message, got {index}"));
                }
                let is_final = count == 1;
                if !is_final && payload.len() != PAYLOAD_PER_FRAME {
                    return Err(format!(
                        "non-final payload must be exactly {PAYLOAD_PER_FRAME}, got {}",
                        payload.len()
                    ));
                }
                if payload.len() > MAX_MESSAGE_BYTES {
                    return Err("payload exceeds message cap".to_string());
                }
                self.max_started = id;
                if is_final {
                    return Ok(Some(payload.to_vec()));
                }
                self.active_id = Some(id);
                self.active_count = count;
                self.next_index = 1;
                self.buf = payload.to_vec();
                Ok(None)
            }
            Some(active) => {
                if id != active {
                    let msg = format!(
                        "new message id {id} while message {active} unfinished (next index {})",
                        self.next_index
                    );
                    self.reset_active();
                    return Err(msg);
                }
                if index != self.next_index {
                    let msg = format!(
                        "expected index {} for message {active}, got {index}",
                        self.next_index
                    );
                    self.reset_active();
                    return Err(msg);
                }
                if count != self.active_count {
                    let msg = format!(
                        "count mismatch for message {active}: expected {}, got {count}",
                        self.active_count
                    );
                    self.reset_active();
                    return Err(msg);
                }
                let is_final = index + 1 == count;
                if !is_final && payload.len() != PAYLOAD_PER_FRAME {
                    let msg = format!(
                        "non-final payload must be exactly {PAYLOAD_PER_FRAME}, got {}",
                        payload.len()
                    );
                    self.reset_active();
                    return Err(msg);
                }
                if self.buf.len().saturating_add(payload.len()) > MAX_MESSAGE_BYTES {
                    self.reset_active();
                    return Err("reassembled message exceeds 524288 bytes".to_string());
                }
                self.buf.extend_from_slice(payload);
                self.next_index += 1;
                if is_final {
                    let done = std::mem::take(&mut self.buf);
                    self.active_id = None;
                    self.active_count = 0;
                    self.next_index = 0;
                    Ok(Some(done))
                } else {
                    Ok(None)
                }
            }
        }
    }
    fn reset_active(&mut self) {
        self.active_id = None;
        self.active_count = 0;
        self.next_index = 0;
        self.buf.clear();
    }
}
