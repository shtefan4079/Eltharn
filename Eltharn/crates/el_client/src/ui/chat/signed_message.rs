#![allow(dead_code)]
use crate::ui::chat::chat_component::GuiMessageTag;
pub const LAST_SEEN_MESSAGES_MAX_LENGTH: usize = 20;
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MessageSignature(pub Vec<u8>);
impl MessageSignature {
    pub fn checksum(&self) -> i32 {
        self.0.iter().fold(1_i32, |hash, byte| {
            hash.wrapping_mul(31).wrapping_add(*byte as i8 as i32)
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedMessageLink {
    pub index: i32,
    pub sender: [u8; 16],
    pub session_id: Option<[u8; 16]>,
}
impl SignedMessageLink {
    pub fn unsigned(sender: [u8; 16]) -> Self {
        Self {
            index: 0,
            sender,
            session_id: None,
        }
    }
    pub fn advance(&self) -> Option<Self> {
        if self.index == i32::MAX {
            None
        } else {
            Some(Self {
                index: self.index + 1,
                sender: self.sender,
                session_id: self.session_id,
            })
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastSeenTrackedEntry {
    pub signature: MessageSignature,
    pub pending: bool,
}
impl LastSeenTrackedEntry {
    pub fn acknowledge(&self) -> Self {
        if self.pending {
            Self {
                signature: self.signature.clone(),
                pending: false,
            }
        } else {
            self.clone()
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastSeenMessages {
    pub entries: Vec<MessageSignature>,
}
impl LastSeenMessages {
    pub const EMPTY: Self = Self {
        entries: Vec::new(),
    };
    pub fn compute_checksum(&self) -> u8 {
        let checksum = self.entries.iter().fold(1_i32, |sum, entry| {
            sum.wrapping_mul(31).wrapping_add(entry.checksum())
        }) as u8;
        if checksum == 0 { 1 } else { checksum }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LastSeenMessagesUpdate {
    pub last_seen: LastSeenMessages,
    pub offset: usize,
    pub acknowledged: Vec<bool>,
    pub checksum: u8,
}
#[derive(Debug, Clone)]
pub struct LastSeenMessagesTracker {
    tracked_messages: Vec<Option<LastSeenTrackedEntry>>,
    tail: usize,
    offset: usize,
    last_tracked_message: Option<MessageSignature>,
    capacity: usize,
}
impl LastSeenMessagesTracker {
    pub fn new(capacity: usize) -> Self {
        Self {
            tracked_messages: vec![None; capacity],
            tail: 0,
            offset: 0,
            last_tracked_message: None,
            capacity,
        }
    }
    pub fn add_pending(&mut self, signature: MessageSignature, was_shown: bool) -> bool {
        if self.last_tracked_message.as_ref() == Some(&signature) {
            return false;
        }
        self.last_tracked_message = Some(signature.clone());
        let entry = was_shown.then_some(LastSeenTrackedEntry {
            signature,
            pending: true,
        });
        self.add_entry(entry);
        true
    }
    pub fn add_seen(&mut self, signature: Option<MessageSignature>) {
        if let Some(sig) = signature {
            self.add_pending(sig, true);
        } else {
            self.add_entry(None);
        }
    }
    fn add_entry(&mut self, entry: Option<LastSeenTrackedEntry>) {
        if self.capacity == 0 {
            return;
        }
        let index = self.tail;
        self.tail = (index + 1) % self.capacity;
        self.offset = self.offset.saturating_add(1);
        self.tracked_messages[index] = entry;
    }
    pub fn ignore_pending(&mut self, pending_message: &MessageSignature) {
        for entry in self.tracked_messages.iter_mut() {
            if let Some(e) = entry {
                if e.pending && &e.signature == pending_message {
                    *entry = None;
                    break;
                }
            }
        }
    }
    pub fn get_and_clear_offset(&mut self) -> usize {
        let original_offset = self.offset;
        self.offset = 0;
        original_offset
    }
    pub fn offset(&self) -> usize {
        self.offset
    }
    pub fn generate_and_apply_update(&mut self) -> LastSeenMessagesUpdate {
        let offset = self.get_and_clear_offset();
        let mut acknowledged = vec![false; self.capacity];
        let mut last_seen_entries = Vec::with_capacity(self.capacity);
        for i in 0..self.capacity {
            let index = (self.tail + i) % self.capacity;
            if let Some(entry) = self.tracked_messages[index].as_mut() {
                acknowledged[i] = true;
                last_seen_entries.push(entry.signature.clone());
                *entry = entry.acknowledge();
            }
        }
        let last_seen = LastSeenMessages {
            entries: last_seen_entries,
        };
        let checksum = last_seen.compute_checksum();
        LastSeenMessagesUpdate {
            last_seen,
            offset,
            acknowledged,
            checksum,
        }
    }
}
impl Default for LastSeenMessagesTracker {
    fn default() -> Self {
        Self::new(LAST_SEEN_MESSAGES_MAX_LENGTH)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerChatMessage {
    pub link: SignedMessageLink,
    pub signature: Option<MessageSignature>,
    pub signed_content: String,
    pub decorated_content: Option<String>,
    pub unsigned: bool,
}
impl PlayerChatMessage {
    pub fn unsigned(sender: [u8; 16], content: impl Into<String>) -> Self {
        Self {
            link: SignedMessageLink::unsigned(sender),
            signature: None,
            signed_content: content.into(),
            decorated_content: None,
            unsigned: true,
        }
    }
    pub fn trust_tag(&self) -> Option<GuiMessageTag> {
        if self.unsigned {
            Some(GuiMessageTag::chat_not_secure())
        } else {
            None
        }
    }
}
