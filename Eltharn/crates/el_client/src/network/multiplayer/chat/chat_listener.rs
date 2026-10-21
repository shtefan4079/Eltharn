#![allow(dead_code)]

use std::collections::VecDeque;
use std::time::Duration;
use web_time::Instant;

use crate::network::multiplayer::chat::{
    ChatLog,
    chat_trust_level::ChatTrustLevel,
    logged_chat_event::{LoggedChatEvent, LoggedSystemMessage},
};

#[derive(Debug)]
struct PendingMessage {
    pub signature: Option<Vec<u8>>,
    pub _content: String,
    pub _is_player_message: bool,
}

impl PendingMessage {
    fn accept(&self) -> bool {
        true
    }
}

pub struct ChatListener {
    message_delay_ms: u64,
    previous_message_time: Option<Instant>,
    delayed_queue: VecDeque<PendingMessage>,
    chat_log: ChatLog,
}

impl ChatListener {
    const DEFAULT_LOG_CAPACITY: usize = 100;

    pub fn new() -> Self {
        ChatListener {
            message_delay_ms: 0,
            previous_message_time: None,
            delayed_queue: VecDeque::new(),
            chat_log: ChatLog::new(Self::DEFAULT_LOG_CAPACITY),
        }
    }

    pub fn tick(&mut self, paused: bool) {
        if paused {
            if self.message_delay_ms > 0 {
                if let Some(ref mut t) = self.previous_message_time {
                    *t += Duration::from_millis(50);
                }
            }
            return;
        }

        if self.message_delay_ms == 0 {
            if !self.delayed_queue.is_empty() {
                self.flush_queue();
            }
        } else if self.should_accept_next() {
            while let Some(msg) = self.delayed_queue.pop_front() {
                if msg.accept() {
                    break;
                }
            }
        }
    }

    pub fn set_message_delay(&mut self, seconds: f64, paused: bool) {
        let new_delay_ms = (seconds * 1000.0) as u64;
        if new_delay_ms == 0 && self.message_delay_ms > 0 && !paused {
            self.flush_queue();
        }
        self.message_delay_ms = new_delay_ms;
    }

    pub fn accept_next_delayed_message(&mut self) {
        if let Some(msg) = self.delayed_queue.pop_front() {
            msg.accept();
        }
    }

    pub fn queue_size(&self) -> usize {
        self.delayed_queue.len()
    }

    pub fn flush_queue(&mut self) {
        for msg in self.delayed_queue.drain(..) {
            msg.accept();
        }
        self.previous_message_time = None;
    }

    pub fn remove_from_delayed_queue(&mut self, signature: &[u8]) -> bool {
        let before = self.delayed_queue.len();
        self.delayed_queue
            .retain(|m| m.signature.as_deref() != Some(signature));
        self.delayed_queue.len() < before
    }

    pub fn handle_player_chat_message(
        &mut self,
        signature: Option<Vec<u8>>,
        content: String,
        _trust_level: ChatTrustLevel,
        can_receive: bool,
    ) {
        if !can_receive {
            return;
        }
        let pending = PendingMessage {
            signature: signature.clone(),
            _content: content.clone(),
            _is_player_message: true,
        };
        self.enqueue_or_accept(pending);
    }

    pub fn handle_disguised_chat_message(&mut self, content: String, can_receive: bool) {
        if !can_receive {
            return;
        }
        let pending = PendingMessage {
            signature: None,
            _content: content,
            _is_player_message: false,
        };
        self.enqueue_or_accept(pending);
    }

    pub fn handle_system_message(&mut self, content: String, remote: bool, can_receive: bool) {
        if !can_receive {
            return;
        }
        if remote {
            let event = LoggedChatEvent::System(LoggedSystemMessage {
                message: content.clone(),
                time_stamp: web_time::Instant::now(),
            });
            self.chat_log.push(event);
        }
    }

    pub fn handle_overlay(&self, _content: &str) {}

    pub fn chat_log(&self) -> &ChatLog {
        &self.chat_log
    }

    pub fn chat_log_mut(&mut self) -> &mut ChatLog {
        &mut self.chat_log
    }

    fn should_accept_next(&self) -> bool {
        if self.message_delay_ms == 0 {
            return true;
        }
        match self.previous_message_time {
            None => true,
            Some(t) => t.elapsed().as_millis() as u64 >= self.message_delay_ms,
        }
    }

    fn enqueue_or_accept(&mut self, msg: PendingMessage) {
        if self.will_delay_messages() {
            self.delayed_queue.push_back(msg);
        } else {
            msg.accept();
            self.previous_message_time = Some(Instant::now());
        }
    }

    fn will_delay_messages(&self) -> bool {
        if self.message_delay_ms == 0 {
            return false;
        }
        match self.previous_message_time {
            None => false,
            Some(t) => (t.elapsed().as_millis() as u64) < self.message_delay_ms,
        }
    }
}

impl Default for ChatListener {
    fn default() -> Self {
        ChatListener::new()
    }
}
