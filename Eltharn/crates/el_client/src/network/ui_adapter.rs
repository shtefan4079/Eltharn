use crate::ui::domain::*;
use crate::ui::gui::hud::*;
use crate::ui::scoreboard::ScoreNumberFormat;
impl From<el_protocol_backend::BackendChatClickAction> for ChatClickAction {
    fn from(value: el_protocol_backend::BackendChatClickAction) -> Self {
        match value {
            el_protocol_backend::BackendChatClickAction::RunCommand { command } => {
                Self::RunCommand { command }
            }
            el_protocol_backend::BackendChatClickAction::SuggestCommand { command } => {
                Self::SuggestCommand { command }
            }
            el_protocol_backend::BackendChatClickAction::CopyToClipboard { value } => {
                Self::CopyToClipboard { value }
            }
            el_protocol_backend::BackendChatClickAction::OpenUrl { url } => Self::OpenUrl { url },
            el_protocol_backend::BackendChatClickAction::OpenFile { path } => {
                Self::OpenFile { path }
            }
            el_protocol_backend::BackendChatClickAction::ChangePage { page } => {
                Self::ChangePage { page }
            }
            el_protocol_backend::BackendChatClickAction::ShowDialog => Self::ShowDialog,
            el_protocol_backend::BackendChatClickAction::Custom { id, payload } => {
                Self::Custom { id, payload }
            }
        }
    }
}
impl From<el_protocol_backend::BackendChatHoverAction> for ChatHoverAction {
    fn from(value: el_protocol_backend::BackendChatHoverAction) -> Self {
        match value {
            el_protocol_backend::BackendChatHoverAction::ShowText { value } => {
                Self::ShowText { value }
            }
            el_protocol_backend::BackendChatHoverAction::ShowItem { id, count, tag } => {
                Self::ShowItem { id, count, tag }
            }
            el_protocol_backend::BackendChatHoverAction::ShowEntity { id, uuid, name } => {
                Self::ShowEntity { id, uuid, name }
            }
        }
    }
}
impl From<el_protocol_backend::BackendChatClickSpan> for ChatClickSpan {
    fn from(value: el_protocol_backend::BackendChatClickSpan) -> Self {
        Self {
            start: value.start,
            end: value.end,
            action: value.action.map(Into::into),
            hover: value.hover.map(Into::into),
            insertion: value.insertion,
        }
    }
}
impl From<el_protocol_backend::BackendNetworkChatSuggestion> for NetworkChatSuggestion {
    fn from(value: el_protocol_backend::BackendNetworkChatSuggestion) -> Self {
        Self {
            text: value.text,
            tooltip: value.tooltip,
        }
    }
}
impl From<el_protocol_backend::BackendCommandNode> for CommandNode {
    fn from(value: el_protocol_backend::BackendCommandNode) -> Self {
        Self {
            executable: value.executable,
            children: value.children,
            redirect_node: value.redirect_node,
            node_type: value.node_type.into(),
            restricted: value.restricted,
        }
    }
}
impl From<el_protocol_backend::BackendCommandParser> for CommandParser {
    fn from(value: el_protocol_backend::BackendCommandParser) -> Self {
        match value {
            el_protocol_backend::BackendCommandParser::Bool => Self::Bool,
            el_protocol_backend::BackendCommandParser::Number { kind, min, max } => Self::Number {
                kind: kind.into(),
                min,
                max,
            },
            el_protocol_backend::BackendCommandParser::String { kind } => {
                Self::String { kind: kind.into() }
            }
            el_protocol_backend::BackendCommandParser::Entity {
                single,
                players_only,
            } => Self::Entity {
                single,
                players_only,
            },
            el_protocol_backend::BackendCommandParser::ScoreHolder { allows_multiple } => {
                Self::ScoreHolder { allows_multiple }
            }
            el_protocol_backend::BackendCommandParser::Time { min } => Self::Time { min },
            el_protocol_backend::BackendCommandParser::Named { kind, registry_key } => {
                Self::Named { kind, registry_key }
            }
        }
    }
}
impl From<el_protocol_backend::BackendCommandNumberKind> for CommandNumberKind {
    fn from(value: el_protocol_backend::BackendCommandNumberKind) -> Self {
        match value {
            el_protocol_backend::BackendCommandNumberKind::Float => Self::Float,
            el_protocol_backend::BackendCommandNumberKind::Double => Self::Double,
            el_protocol_backend::BackendCommandNumberKind::Integer => Self::Integer,
            el_protocol_backend::BackendCommandNumberKind::Long => Self::Long,
        }
    }
}
impl From<el_protocol_backend::BackendCommandStringKind> for CommandStringKind {
    fn from(value: el_protocol_backend::BackendCommandStringKind) -> Self {
        match value {
            el_protocol_backend::BackendCommandStringKind::SingleWord => Self::SingleWord,
            el_protocol_backend::BackendCommandStringKind::QuotablePhrase => Self::QuotablePhrase,
            el_protocol_backend::BackendCommandStringKind::GreedyPhrase => Self::GreedyPhrase,
        }
    }
}
impl From<el_protocol_backend::BackendCommandNodeType> for CommandNodeType {
    fn from(value: el_protocol_backend::BackendCommandNodeType) -> Self {
        match value {
            el_protocol_backend::BackendCommandNodeType::Root => Self::Root,
            el_protocol_backend::BackendCommandNodeType::Literal { name } => Self::Literal { name },
            el_protocol_backend::BackendCommandNodeType::Argument {
                name,
                parser,
                parser_info,
                suggestions_type,
            } => Self::Argument {
                name,
                parser,
                parser_info: parser_info.into(),
                suggestions_type,
            },
        }
    }
}
impl From<el_protocol_backend::BackendChatCompletionAction> for ChatCompletionAction {
    fn from(value: el_protocol_backend::BackendChatCompletionAction) -> Self {
        match value {
            el_protocol_backend::BackendChatCompletionAction::Add => Self::Add,
            el_protocol_backend::BackendChatCompletionAction::Remove => Self::Remove,
            el_protocol_backend::BackendChatCompletionAction::Set => Self::Set,
        }
    }
}
impl From<el_protocol_backend::BackendPlayerInfoAddPlayer> for PlayerInfoAddPlayer {
    fn from(value: el_protocol_backend::BackendPlayerInfoAddPlayer) -> Self {
        Self {
            name: value.name,
            texture_url: value.texture_url,
            model: value.model,
            cape_url: value.cape_url,
        }
    }
}
impl From<el_protocol_backend::BackendPlayerInfoDisplayNameUpdate> for PlayerInfoDisplayNameUpdate {
    fn from(value: el_protocol_backend::BackendPlayerInfoDisplayNameUpdate) -> Self {
        match value {
            el_protocol_backend::BackendPlayerInfoDisplayNameUpdate::Clear => Self::Clear,
            el_protocol_backend::BackendPlayerInfoDisplayNameUpdate::Set { text } => {
                Self::Set { text }
            }
        }
    }
}
impl From<el_protocol_backend::BackendPlayerInfoUpdateEntry> for PlayerInfoUpdateEntry {
    fn from(value: el_protocol_backend::BackendPlayerInfoUpdateEntry) -> Self {
        Self {
            uuid: value.uuid,
            add_player: value.add_player.map(Into::into),
            game_mode: value.game_mode,
            listed: value.listed,
            latency: value.latency,
            display_name: value.display_name.map(Into::into),
            list_order: value.list_order,
            show_hat: value.show_hat,
        }
    }
}
impl From<el_protocol_backend::BackendBossBarColor> for BossBarColor {
    fn from(c: el_protocol_backend::BackendBossBarColor) -> Self {
        match c {
            el_protocol_backend::BackendBossBarColor::Pink => Self::Pink,
            el_protocol_backend::BackendBossBarColor::Blue => Self::Blue,
            el_protocol_backend::BackendBossBarColor::Red => Self::Red,
            el_protocol_backend::BackendBossBarColor::Green => Self::Green,
            el_protocol_backend::BackendBossBarColor::Yellow => Self::Yellow,
            el_protocol_backend::BackendBossBarColor::Purple => Self::Purple,
            el_protocol_backend::BackendBossBarColor::White => Self::White,
        }
    }
}
impl From<el_protocol_backend::BackendBossBarOverlay> for BossBarOverlay {
    fn from(o: el_protocol_backend::BackendBossBarOverlay) -> Self {
        match o {
            el_protocol_backend::BackendBossBarOverlay::Progress => Self::Progress,
            el_protocol_backend::BackendBossBarOverlay::Notched6 => Self::Notched6,
            el_protocol_backend::BackendBossBarOverlay::Notched10 => Self::Notched10,
            el_protocol_backend::BackendBossBarOverlay::Notched12 => Self::Notched12,
            el_protocol_backend::BackendBossBarOverlay::Notched20 => Self::Notched20,
        }
    }
}
impl From<el_protocol_backend::BackendBossEventStyle> for BossEventStyle {
    fn from(s: el_protocol_backend::BackendBossEventStyle) -> Self {
        Self {
            color: s.color.into(),
            overlay: s.overlay.into(),
        }
    }
}
impl From<el_protocol_backend::BackendBossEventProperties> for BossEventProperties {
    fn from(p: el_protocol_backend::BackendBossEventProperties) -> Self {
        Self {
            darken_screen: p.darken_screen,
            play_music: p.play_music,
            create_world_fog: p.create_world_fog,
        }
    }
}
impl From<el_protocol_backend::BackendBossEventAdd> for BossEventAdd {
    fn from(a: el_protocol_backend::BackendBossEventAdd) -> Self {
        Self {
            name: a.name,
            progress: a.progress,
            style: a.style.into(),
            properties: a.properties.into(),
        }
    }
}
impl From<el_protocol_backend::BackendBossEventOperation> for BossEventOperation {
    fn from(op: el_protocol_backend::BackendBossEventOperation) -> Self {
        match op {
            el_protocol_backend::BackendBossEventOperation::Add(add) => Self::Add(add.into()),
            el_protocol_backend::BackendBossEventOperation::Remove => Self::Remove,
            el_protocol_backend::BackendBossEventOperation::UpdateProgress(p) => {
                Self::UpdateProgress(p)
            }
            el_protocol_backend::BackendBossEventOperation::UpdateName(n) => Self::UpdateName(n),
            el_protocol_backend::BackendBossEventOperation::UpdateStyle(s) => {
                Self::UpdateStyle(s.into())
            }
            el_protocol_backend::BackendBossEventOperation::UpdateProperties(p) => {
                Self::UpdateProperties(p.into())
            }
        }
    }
}
impl From<el_protocol_backend::BackendScoreNumberFormat> for ScoreNumberFormat {
    fn from(f: el_protocol_backend::BackendScoreNumberFormat) -> Self {
        match f {
            el_protocol_backend::BackendScoreNumberFormat::Blank => Self::Blank,
            el_protocol_backend::BackendScoreNumberFormat::Styled { color, style } => {
                Self::Styled { color, style }
            }
            el_protocol_backend::BackendScoreNumberFormat::Fixed { text } => Self::Fixed { text },
        }
    }
}
