use std::collections::HashSet;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatRestriction {
    ChatAndCommandsDisabledByOptions,
    ChatDisabledByOptions,
    DisabledByLauncher,
    DisabledByProfile,
}

#[allow(dead_code)]
impl ChatRestriction {
    pub fn translation_key(self) -> &'static str {
        match self {
            ChatRestriction::ChatAndCommandsDisabledByOptions => {
                "chat_restriction.chat_and_commands_disabled_by_options"
            }
            ChatRestriction::ChatDisabledByOptions => "chat_restriction.chat_disabled_by_options",
            ChatRestriction::DisabledByLauncher => "chat_restriction.disabled_by_launcher",
            ChatRestriction::DisabledByProfile => "chat_restriction.disabled_by_profile",
        }
    }

    pub fn modify_permissions(self, permissions: &mut HashSet<ChatPermission>) {
        match self {
            ChatRestriction::ChatAndCommandsDisabledByOptions => {
                permissions.remove(&ChatPermission::SendCommands);
                permissions.remove(&ChatPermission::SendMessages);
                permissions.remove(&ChatPermission::ReceivePlayerMessages);
                permissions.remove(&ChatPermission::ReceiveSystemMessages);
            }
            ChatRestriction::ChatDisabledByOptions
            | ChatRestriction::DisabledByLauncher
            | ChatRestriction::DisabledByProfile => {
                permissions.remove(&ChatPermission::SendMessages);
                permissions.remove(&ChatPermission::ReceivePlayerMessages);
            }
        }
    }

    pub fn action(self) -> Option<ChatRestrictionAction> {
        match self {
            ChatRestriction::ChatAndCommandsDisabledByOptions
            | ChatRestriction::ChatDisabledByOptions => {
                Some(ChatRestrictionAction::GoToChatSettings)
            }
            ChatRestriction::DisabledByProfile => Some(ChatRestrictionAction::GoToAccount),
            ChatRestriction::DisabledByLauncher => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChatPermission {
    SendMessages,
    SendCommands,
    ReceivePlayerMessages,
    ReceiveSystemMessages,
}

impl ChatPermission {
    pub fn all() -> HashSet<ChatPermission> {
        [
            ChatPermission::SendMessages,
            ChatPermission::SendCommands,
            ChatPermission::ReceivePlayerMessages,
            ChatPermission::ReceiveSystemMessages,
        ]
        .into_iter()
        .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatRestrictionAction {
    GoToAccount,
    GoToChatSettings,
}
