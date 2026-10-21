use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatClickAction {
    RunCommand {
        command: String,
    },
    SuggestCommand {
        command: String,
    },
    CopyToClipboard {
        value: String,
    },
    OpenUrl {
        url: String,
    },
    OpenFile {
        path: String,
    },
    ChangePage {
        page: i32,
    },
    ShowDialog,
    Custom {
        id: String,
        payload: Option<Vec<u8>>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChatHoverAction {
    ShowText {
        value: String,
    },
    ShowItem {
        id: String,
        count: u32,
        tag: Option<Vec<u8>>,
    },
    ShowEntity {
        id: String,
        uuid: String,
        name: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatClickSpan {
    pub start: usize,
    pub end: usize,
    pub action: Option<ChatClickAction>,
    pub hover: Option<ChatHoverAction>,
    pub insertion: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetworkChatSuggestion {
    pub text: String,
    pub tooltip: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandNode {
    pub executable: bool,
    pub children: Vec<u32>,
    pub redirect_node: Option<u32>,
    pub node_type: CommandNodeType,
    pub restricted: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandParser {
    Bool,
    Number {
        kind: CommandNumberKind,
        min: Option<String>,
        max: Option<String>,
    },
    String {
        kind: CommandStringKind,
    },
    Entity {
        single: bool,
        players_only: bool,
    },
    ScoreHolder {
        allows_multiple: bool,
    },
    Time {
        min: i32,
    },
    Named {
        kind: String,
        registry_key: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandNumberKind {
    Float,
    Double,
    Integer,
    Long,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandStringKind {
    SingleWord,
    QuotablePhrase,
    GreedyPhrase,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandNodeType {
    Root,
    Literal {
        name: String,
    },
    Argument {
        name: String,
        parser: String,
        parser_info: CommandParser,
        suggestions_type: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub enum ChatCompletionAction {
    Add,
    Remove,
    Set,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerInfoAddPlayer {
    pub name: String,
    pub texture_url: Option<String>,
    pub model: Option<String>,
    pub cape_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PlayerInfoDisplayNameUpdate {
    Clear,
    Set { text: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayerInfoUpdateEntry {
    pub uuid: Uuid,
    pub add_player: Option<PlayerInfoAddPlayer>,
    pub game_mode: Option<u8>,
    pub listed: Option<bool>,
    pub latency: Option<i32>,
    pub display_name: Option<PlayerInfoDisplayNameUpdate>,
    pub list_order: Option<i32>,
    pub show_hat: Option<bool>,
}

impl Default for CommandParser {
    fn default() -> Self {
        Self::Named {
            kind: "unknown".to_owned(),
            registry_key: None,
        }
    }
}
