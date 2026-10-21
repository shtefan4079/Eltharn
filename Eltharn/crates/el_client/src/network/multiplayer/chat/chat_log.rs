use crate::network::multiplayer::chat::logged_chat_event::LoggedChatEvent;
pub struct ChatLog {
    buffer: Vec<Option<LoggedChatEvent>>,
    next_id: usize,
}
#[allow(dead_code)]
impl ChatLog {
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "ChatLog capacity must be > 0");
        ChatLog {
            buffer: vec![None; capacity],
            next_id: 0,
        }
    }
    pub fn push(&mut self, event: LoggedChatEvent) {
        let idx = self.index(self.next_id);
        self.buffer[idx] = Some(event);
        self.next_id += 1;
    }
    pub fn lookup(&self, id: usize) -> Option<&LoggedChatEvent> {
        if id >= self.start() && id <= self.end() {
            self.buffer[self.index(id)].as_ref()
        } else {
            None
        }
    }
    pub fn start(&self) -> usize {
        self.next_id.saturating_sub(self.buffer.len())
    }
    pub fn end(&self) -> usize {
        if self.next_id == 0 {
            0
        } else {
            self.next_id - 1
        }
    }
    fn index(&self, id: usize) -> usize {
        id % self.buffer.len()
    }
    pub fn size(&self) -> usize {
        if self.next_id == 0 {
            0
        } else {
            self.end() - self.start() + 1
        }
    }
    pub fn is_empty(&self) -> bool {
        self.next_id == 0
    }
    pub fn iter(&self) -> impl Iterator<Item = &LoggedChatEvent> {
        let range = if self.is_empty() {
            1..=0
        } else {
            self.start()..=self.end()
        };
        range.filter_map(|id| self.lookup(id))
    }
}
