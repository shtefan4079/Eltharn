#![allow(dead_code)]

use el_core::UUID;
use web_time::Instant;

use crate::network::multiplayer::chat::chat_trust_level::ChatTrustLevel;

#[derive(Debug, Clone)]
pub enum LoggedChatEvent {
    Player(LoggedPlayerMessage),
    System(LoggedSystemMessage),
}

impl LoggedChatEvent {
    pub fn event_type(&self) -> LoggedChatEventType {
        match self {
            LoggedChatEvent::Player(_) => LoggedChatEventType::Player,
            LoggedChatEvent::System(_) => LoggedChatEventType::System,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoggedChatEventType {
    Player,
    System,
}

impl LoggedChatEventType {
    pub fn serialized_name(self) -> &'static str {
        match self {
            LoggedChatEventType::Player => "player",
            LoggedChatEventType::System => "system",
        }
    }
}

pub trait LoggedChatMessage {
    fn to_content(&self) -> String;
    fn can_report(&self, reported_player_id: UUID) -> bool;
}

#[derive(Debug, Clone)]
pub struct LoggedPlayerMessage {
    pub profile_name: String,
    pub profile_id: UUID,
    pub signed_content: String,
    pub decorated_content: String,
    pub has_signature: bool,
    pub sender: UUID,
    pub trust_level: ChatTrustLevel,
}

impl LoggedChatMessage for LoggedPlayerMessage {
    fn to_content(&self) -> String {
        self.decorated_content.clone()
    }

    fn can_report(&self, reported_player_id: UUID) -> bool {
        self.has_signature && self.sender == reported_player_id
    }
}

#[derive(Debug, Clone)]
pub struct LoggedSystemMessage {
    pub message: String,
    pub time_stamp: Instant,
}

impl LoggedChatMessage for LoggedSystemMessage {
    fn to_content(&self) -> String {
        self.message.clone()
    }

    fn can_report(&self, _: UUID) -> bool {
        false
    }
}
