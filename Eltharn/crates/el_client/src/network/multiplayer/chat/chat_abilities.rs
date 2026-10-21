#![allow(dead_code)]
use crate::network::multiplayer::chat::{
    GuiMessage, GuiMessageSource,
    chat_restriction::{ChatPermission, ChatRestriction},
};
use std::collections::HashSet;
#[derive(Debug, Clone)]
pub struct ChatAbilities {
    restriction_reasons: HashSet<ChatRestriction>,
    permissions: HashSet<ChatPermission>,
}
#[allow(dead_code)]
impl ChatAbilities {
    pub fn no_restrictions() -> Self {
        Self::from_restrictions(HashSet::new())
    }
    pub fn from_restrictions(restrictions: HashSet<ChatRestriction>) -> Self {
        let mut permissions = ChatPermission::all();
        for r in &restrictions {
            r.modify_permissions(&mut permissions);
        }
        ChatAbilities {
            restriction_reasons: restrictions,
            permissions,
        }
    }
    pub fn has_any_restrictions(&self) -> bool {
        !self.restriction_reasons.is_empty()
    }
    pub fn restrictions(&self) -> impl Iterator<Item = &ChatRestriction> {
        self.restriction_reasons.iter()
    }
    pub fn can_send_messages(&self) -> bool {
        self.permissions.contains(&ChatPermission::SendMessages)
    }
    pub fn can_send_commands(&self) -> bool {
        self.permissions.contains(&ChatPermission::SendCommands)
    }
    pub fn can_receive_player_messages(&self) -> bool {
        self.permissions
            .contains(&ChatPermission::ReceivePlayerMessages)
    }
    pub fn can_receive_system_messages(&self) -> bool {
        self.permissions
            .contains(&ChatPermission::ReceiveSystemMessages)
    }
    pub fn is_message_visible(&self, message: &GuiMessage) -> bool {
        match message.source {
            GuiMessageSource::SystemClient => true,
            GuiMessageSource::Player => self.can_receive_player_messages(),
            GuiMessageSource::SystemServer => self.can_receive_system_messages(),
        }
    }
}
#[derive(Debug, Default)]
pub struct ChatAbilitiesBuilder {
    restrictions: HashSet<ChatRestriction>,
}
impl ChatAbilitiesBuilder {
    pub fn new() -> Self {
        ChatAbilitiesBuilder::default()
    }
    pub fn add_restriction(mut self, r: ChatRestriction) -> Self {
        self.restrictions.insert(r);
        self
    }
    pub fn build(self) -> ChatAbilities {
        ChatAbilities::from_restrictions(self.restrictions)
    }
}
