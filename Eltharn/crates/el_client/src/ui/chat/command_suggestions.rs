use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatSuggestion {
    pub display: String,
    pub replacement: String,
    pub range_start: usize,
    pub range_end: usize,
    pub tooltip: Option<String>,
}
#[allow(dead_code)]
impl ChatSuggestion {
    pub fn new(display: String, replacement: String, range_start: usize, range_end: usize) -> Self {
        Self {
            display,
            replacement,
            range_start,
            range_end,
            tooltip: None,
        }
    }
    pub fn with_tooltip(mut self, tooltip: String) -> Self {
        self.tooltip = Some(tooltip);
        self
    }
    pub fn apply(&self, original: &str) -> String {
        let start = byte_index(original, self.range_start);
        let end = byte_index(original, self.range_end);
        let mut applied = String::with_capacity(
            original.len()
                + self
                    .replacement
                    .len()
                    .saturating_sub(end.saturating_sub(start)),
        );
        applied.push_str(&original[..start]);
        applied.push_str(&self.replacement);
        applied.push_str(&original[end..]);
        applied
    }
    pub fn replacement_end(&self) -> usize {
        self.range_start + self.replacement.chars().count()
    }
}
pub struct CommandSuggestions;
#[allow(dead_code)]
impl CommandSuggestions {
    pub const LINE_HEIGHT: f32 = 12.0;
    pub const USAGE_OFFSET_FROM_BOTTOM: f32 = 27.0;
    pub const DEFAULT_LINE_LIMIT: usize = 10;
    pub const DEFAULT_FILL_COLOR: u32 = 0xD0000000;
    pub fn build(text: &str, cursor: usize) -> Vec<ChatSuggestion> {
        build_chat_suggestions(text, cursor)
    }
    pub fn build_with_server_commands(
        text: &str,
        cursor: usize,
        server_commands: &[String],
    ) -> Vec<ChatSuggestion> {
        build_chat_suggestions_with_commands(text, cursor, server_commands)
    }
    pub fn build_custom_chat_completions(
        text: &str,
        cursor: usize,
        custom_completions: &[String],
    ) -> Vec<ChatSuggestion> {
        build_custom_chat_completions(text, cursor, custom_completions)
    }
}
pub fn format_chat_input_like_vanilla(
    text: &str,
    visible_start: usize,
    visible_end: usize,
    root_index: u32,
    nodes: &[crate::ui::domain::CommandNode],
) -> Option<String> {
    if !text.starts_with('/') || nodes.is_empty() {
        return None;
    }
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum InputStyle {
        Literal,
        Argument(usize),
        Unparsed,
    }
    const ARGUMENT_COLORS: [[u8; 3]; 5] = [
        [85, 255, 255],
        [255, 255, 85],
        [85, 255, 85],
        [255, 85, 255],
        [255, 170, 0],
    ];
    const LITERAL_COLOR: [u8; 3] = [170, 170, 170];
    const UNPARSED_COLOR: [u8; 3] = [255, 85, 85];
    let chars = text.chars().collect::<Vec<_>>();
    let mut styles = vec![InputStyle::Literal; chars.len()];
    let mut current_node = root_index as usize;
    let mut next_argument_style = 0usize;
    let body = &text[1..];
    let spans = command_token_spans(body);
    let mut span_index = 0usize;
    while span_index < spans.len() {
        let (start_byte, end_byte) = spans[span_index];
        let start = 1 + body[..start_byte].chars().count();
        let mut end_byte = end_byte;
        let token = &body[start_byte..end_byte];
        let Some((child_index, is_argument)) = command_child(nodes, current_node, token) else {
            for style in styles.iter_mut().skip(start) {
                *style = InputStyle::Unparsed;
            }
            break;
        };
        let token_count = command_argument_token_count(nodes, child_index);
        if token_count > 1 {
            let last_span = (span_index + token_count - 1).min(spans.len() - 1);
            end_byte = spans[last_span].1;
        }
        let end = 1 + body[..end_byte].chars().count();
        let style = if is_argument {
            let style = InputStyle::Argument(next_argument_style);
            next_argument_style = next_argument_style.wrapping_add(1);
            style
        } else {
            InputStyle::Literal
        };
        for slot in styles.iter_mut().take(end).skip(start) {
            *slot = style;
        }
        current_node = child_index;
        if is_greedy_argument(nodes, child_index) {
            for slot in styles.iter_mut().skip(end) {
                *slot = style;
            }
            break;
        }
        span_index += token_count.max(1);
    }
    let start = visible_start.min(chars.len());
    let end = visible_end.min(chars.len()).max(start);
    let mut formatted = String::new();
    let mut run_start = start;
    while run_start < end {
        let style = styles[run_start];
        let mut run_end = run_start + 1;
        while run_end < end && styles[run_end] == style {
            run_end += 1;
        }
        let color = match style {
            InputStyle::Literal => LITERAL_COLOR,
            InputStyle::Argument(style) => ARGUMENT_COLORS[style % ARGUMENT_COLORS.len()],
            InputStyle::Unparsed => UNPARSED_COLOR,
        };
        formatted.push_str(&format!(
            "\u{1b}[38;2;{};{};{}m",
            color[0], color[1], color[2]
        ));
        formatted.extend(chars[run_start..run_end].iter());
        run_start = run_end;
    }
    formatted.push_str("\u{1b}[m");
    Some(formatted)
}
pub fn build_chat_suggestions_with_command_nodes(
    text: &str,
    cursor: usize,
    root_index: u32,
    nodes: &[crate::ui::domain::CommandNode],
) -> Vec<ChatSuggestion> {
    let cursor = cursor.min(text.chars().count());
    let cursor_byte = byte_index(text, cursor);
    let before_cursor = &text[..cursor_byte];
    if !before_cursor.starts_with('/') || nodes.is_empty() {
        return Vec::new();
    }
    let body = &before_cursor[1..];
    let word_start_in_body = get_last_word_index(body);
    let word_start_byte = 1 + word_start_in_body;
    let word_start = before_cursor[..word_start_byte].chars().count();
    let prefix = &before_cursor[word_start_byte..];
    let Some(current_node) =
        command_context_node(nodes, root_index as usize, body, word_start_in_body)
    else {
        return Vec::new();
    };
    let mut suggestions = Vec::new();
    for child_index in command_children(nodes, current_node) {
        let Some(child) = nodes.get(child_index as usize) else {
            continue;
        };
        let crate::ui::domain::CommandNodeType::Literal { name } = &child.node_type else {
            continue;
        };
        if name.starts_with(prefix) || name.to_lowercase().starts_with(&prefix.to_lowercase()) {
            suggestions.push(ChatSuggestion::new(
                name.clone(),
                name.clone(),
                word_start,
                cursor,
            ));
        }
    }
    sort_like_vanilla(&mut suggestions, prefix);
    suggestions
}
pub fn command_usage_for_input(
    text: &str,
    cursor: usize,
    root_index: u32,
    nodes: &[crate::ui::domain::CommandNode],
) -> Vec<(u32, String)> {
    let cursor = cursor.min(text.chars().count());
    let cursor_byte = byte_index(text, cursor);
    let before_cursor = &text[..cursor_byte];
    let Some(body) = before_cursor.strip_prefix('/') else {
        return Vec::new();
    };
    if nodes.is_empty() {
        return Vec::new();
    }
    let word_start = get_last_word_index(body);
    let Some(current) = command_context_node(nodes, root_index as usize, body, word_start) else {
        return Vec::new();
    };
    let Some(parent) = nodes.get(current) else {
        return Vec::new();
    };
    let optional = parent.executable;
    parent
        .children
        .iter()
        .filter_map(|child| {
            let child = *child as usize;
            let usage = smart_usage_recursive(nodes, child, optional, false, root_index as usize)?;
            Some((child as u32, usage))
        })
        .collect()
}
fn smart_usage_recursive(
    nodes: &[crate::ui::domain::CommandNode],
    node_index: usize,
    optional: bool,
    deep: bool,
    root_index: usize,
) -> Option<String> {
    let node = nodes.get(node_index)?;
    let usage_text = match &node.node_type {
        crate::ui::domain::CommandNodeType::Root => return None,
        crate::ui::domain::CommandNodeType::Literal { name } => name.clone(),
        crate::ui::domain::CommandNodeType::Argument { name, .. } => format!("<{name}>"),
    };
    let this = if optional {
        format!("[{usage_text}]")
    } else {
        usage_text
    };
    let child_optional = node.executable;
    let (open, close) = if child_optional {
        ('[', ']')
    } else {
        ('(', ')')
    };
    if deep {
        return Some(this);
    }
    if let Some(redirect) = node.redirect_node {
        let redirect = if redirect as usize == root_index {
            "...".to_owned()
        } else {
            let target = nodes.get(redirect as usize)?;
            target_name(target)?
        };
        return Some(format!("{this} -> {redirect}"));
    }
    let children = node
        .children
        .iter()
        .copied()
        .filter(|child| nodes.get(*child as usize).is_some())
        .collect::<Vec<_>>();
    match children.len() {
        0 => {}
        1 => {
            if let Some(usage) = smart_usage_recursive(
                nodes,
                children[0] as usize,
                child_optional,
                child_optional,
                root_index,
            ) {
                return Some(format!("{this} {usage}"));
            }
        }
        _ => {
            let mut unique = BTreeSet::new();
            for child in &children {
                if let Some(usage) =
                    smart_usage_recursive(nodes, *child as usize, child_optional, true, root_index)
                {
                    unique.insert(usage);
                }
            }
            match unique.len() {
                0 => {}
                1 => {
                    let usage = unique.into_iter().next().unwrap_or_default();
                    let usage = if child_optional {
                        format!("[{usage}]")
                    } else {
                        usage
                    };
                    return Some(format!("{this} {usage}"));
                }
                2.. => {
                    let alternatives = children
                        .iter()
                        .filter_map(|child| target_name(nodes.get(*child as usize)?))
                        .collect::<Vec<_>>();
                    if !alternatives.is_empty() {
                        return Some(format!("{this} {open}{}{close}", alternatives.join("|")));
                    }
                }
            }
        }
    }
    Some(this)
}
fn target_name(node: &crate::ui::domain::CommandNode) -> Option<String> {
    match &node.node_type {
        crate::ui::domain::CommandNodeType::Root => None,
        crate::ui::domain::CommandNodeType::Literal { name } => Some(name.clone()),
        crate::ui::domain::CommandNodeType::Argument { name, .. } => Some(format!("<{name}>")),
    }
}
fn command_argument_accepts_token(parser: &crate::ui::domain::CommandParser, token: &str) -> bool {
    match parser {
        crate::ui::domain::CommandParser::Bool => matches!(token, "true" | "false"),
        crate::ui::domain::CommandParser::Number { kind, min, max } => match kind {
            crate::ui::domain::CommandNumberKind::Float => {
                number_in_bounds::<f32>(token, min.as_deref(), max.as_deref())
            }
            crate::ui::domain::CommandNumberKind::Double => {
                number_in_bounds::<f64>(token, min.as_deref(), max.as_deref())
            }
            crate::ui::domain::CommandNumberKind::Integer => {
                number_in_bounds::<i32>(token, min.as_deref(), max.as_deref())
            }
            crate::ui::domain::CommandNumberKind::Long => {
                number_in_bounds::<i64>(token, min.as_deref(), max.as_deref())
            }
        },
        crate::ui::domain::CommandParser::String { kind } => match kind {
            crate::ui::domain::CommandStringKind::SingleWord => {
                !token.is_empty() && token.chars().all(is_allowed_unquoted_string_char)
            }
            crate::ui::domain::CommandStringKind::QuotablePhrase => is_valid_quotable_phrase(token),
            crate::ui::domain::CommandStringKind::GreedyPhrase => true,
        },
        crate::ui::domain::CommandParser::Time { min } => time_in_bounds(token, *min),
        crate::ui::domain::CommandParser::Entity { .. }
        | crate::ui::domain::CommandParser::ScoreHolder { .. } => true,
        crate::ui::domain::CommandParser::Named {
            kind,
            registry_key: _,
        } => named_parser_accepts_token(kind, token),
    }
}
fn named_parser_accepts_token(kind: &str, token: &str) -> bool {
    let kind = kind.split('{').next().unwrap_or(kind).trim();
    match kind {
        "GameMode" => {
            matches!(token, "survival" | "creative" | "adventure" | "spectator")
        }
        "HexColor" => {
            (token.len() == 3 || token.len() == 6)
                && token.chars().all(|character| character.is_ascii_hexdigit())
        }
        "EntityAnchor" => matches!(token, "feet" | "eyes"),
        "ScoreboardSlot" => {
            matches!(
                token,
                "list"
                    | "sidebar"
                    | "below_name"
                    | "sidebar.team.black"
                    | "sidebar.team.dark_blue"
                    | "sidebar.team.dark_green"
                    | "sidebar.team.dark_aqua"
                    | "sidebar.team.dark_red"
                    | "sidebar.team.dark_purple"
                    | "sidebar.team.gold"
                    | "sidebar.team.gray"
                    | "sidebar.team.dark_gray"
                    | "sidebar.team.blue"
                    | "sidebar.team.green"
                    | "sidebar.team.aqua"
                    | "sidebar.team.red"
                    | "sidebar.team.light_purple"
                    | "sidebar.team.yellow"
                    | "sidebar.team.white"
            )
        }
        "Swizzle" => {
            !token.is_empty()
                && token.len() <= 3
                && token
                    .chars()
                    .all(|character| matches!(character, 'x' | 'y' | 'z'))
                && token
                    .chars()
                    .all(|character| token.matches(character).count() == 1)
        }
        "Operation" => {
            matches!(
                token,
                "=" | "+=" | "-=" | "*=" | "/=" | "%=" | "<" | ">" | "><"
            )
        }
        "Uuid" => uuid::Uuid::parse_str(token).is_ok(),
        "Identifier" => valid_identifier(token),
        "Dimension" | "Team" | "Objective" | "ObjectiveCriteria" | "Function" => {
            valid_identifier(token)
        }
        "IntRange" => valid_range(token, false),
        "FloatRange" => valid_range(token, true),
        "Angle" => valid_angle(token),
        "Vec2" | "ColumnPos" | "Rotation" => valid_coordinate_prefix(token, 2, false),
        "Vec3" => valid_coordinate_prefix(token, 3, false),
        "BlockPos" => valid_coordinate_prefix(token, 3, true),
        "Entity" => valid_entity_selector(token, true, false),
        "GameProfile" => !token.is_empty() && token.chars().all(is_allowed_unquoted_string_char),
        "ResourceOrTag" | "ResourceOrTagKey" | "resource_or_tag" | "resource_or_tag_key" => {
            valid_resource_or_tag(token)
        }
        "Resource" | "ResourceKey" | "resource" | "resource_key" => valid_identifier(token),
        "ResourceSelector" | "resource_selector" => valid_resource_pattern(token),
        _ => true,
    }
}
fn valid_identifier(token: &str) -> bool {
    if token.is_empty()
        || token
            .chars()
            .any(|character| !is_allowed_identifier_char(character))
    {
        return false;
    }
    let mut parts = token.splitn(2, ':');
    let first = parts.next().unwrap_or_default();
    let Some(path) = parts.next() else {
        return !first.is_empty() && first.chars().all(is_allowed_identifier_path_char);
    };
    !first.is_empty()
        && !path.is_empty()
        && first.chars().all(is_allowed_identifier_namespace_char)
        && path.chars().all(is_allowed_identifier_path_char)
}
fn valid_resource_or_tag(token: &str) -> bool {
    token
        .strip_prefix('#')
        .map_or_else(|| valid_identifier(token), valid_identifier)
}
fn valid_resource_pattern(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|character| {
            is_allowed_identifier_char(character) || matches!(character, '*' | '?')
        })
}
fn is_allowed_identifier_char(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '_' | ':' | '/' | '.' | '-')
}
fn is_allowed_identifier_namespace_char(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || matches!(character, '_' | '-' | '.')
}
fn is_allowed_identifier_path_char(character: char) -> bool {
    is_allowed_identifier_namespace_char(character) || character == '/'
}
fn valid_range(token: &str, floating_point: bool) -> bool {
    let mut parts = token.split("..");
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    if parts.next().is_some() || (first.is_empty() && second.unwrap_or_default().is_empty()) {
        return false;
    }
    let valid_number = |value: &str| {
        if value.is_empty() {
            return true;
        }
        if floating_point {
            value.parse::<f64>().is_ok()
        } else {
            value.parse::<i32>().is_ok()
        }
    };
    valid_number(first) && second.map_or(true, valid_number)
}
fn valid_angle(token: &str) -> bool {
    let value = token.strip_prefix('~').unwrap_or(token);
    value.is_empty() || value.parse::<f32>().is_ok_and(|number| number.is_finite())
}
fn valid_entity_selector(token: &str, single: bool, players_only: bool) -> bool {
    if token.is_empty() {
        return false;
    }
    if !token.starts_with('@') {
        return token.chars().all(is_allowed_unquoted_string_char)
            || uuid::Uuid::parse_str(token).is_ok();
    }
    let Some(selector) = token.chars().nth(1) else {
        return false;
    };
    if !matches!(selector, 'p' | 'a' | 'r' | 's' | 'e' | 'n') {
        return false;
    }
    let has_options = token.ends_with(']');
    if token.contains('[') != has_options {
        return false;
    }
    if has_options {
        let Some(open) = token.find('[') else {
            return false;
        };
        if open != 2 || token.len() <= 4 || !token[3..token.len() - 1].contains('=') {
            return false;
        }
    } else if token.len() != 2 {
        return false;
    }
    if players_only && matches!(selector, 'e' | 'n') {
        return false;
    }
    if single && matches!(selector, 'a' | 'r' | 'e') {
        return false;
    }
    true
}
fn number_in_bounds<T>(token: &str, min: Option<&str>, max: Option<&str>) -> bool
where
    T: std::str::FromStr + PartialOrd,
{
    if token.is_empty() || !token.chars().all(is_allowed_number_char) {
        return false;
    }
    let Ok(value) = token.parse::<T>() else {
        return false;
    };
    if let Some(min) = min {
        let Ok(min) = min.parse::<T>() else {
            return false;
        };
        if value < min {
            return false;
        }
    }
    if let Some(max) = max {
        let Ok(max) = max.parse::<T>() else {
            return false;
        };
        if value > max {
            return false;
        }
    }
    true
}
fn is_allowed_number_char(character: char) -> bool {
    character.is_ascii_digit() || matches!(character, '.' | '-')
}
fn is_allowed_unquoted_string_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | '.' | '+')
}
fn is_valid_quotable_phrase(token: &str) -> bool {
    let Some(first) = token.chars().next() else {
        return false;
    };
    if !matches!(first, '"' | '\'') {
        return token.chars().all(is_allowed_unquoted_string_char);
    }
    let mut escaped = false;
    let mut characters = token.chars();
    let quote = characters.next().unwrap_or(first);
    while let Some(character) = characters.next() {
        if escaped {
            if character != quote && character != '\\' {
                return false;
            }
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == quote {
            return characters.next().is_none();
        }
    }
    false
}
fn time_in_bounds(token: &str, minimum: i32) -> bool {
    let split = token
        .char_indices()
        .find(|(_, character)| !is_allowed_number_char(*character))
        .map(|(index, _)| index)
        .unwrap_or(token.len());
    let (number, unit) = token.split_at(split);
    if number.is_empty() || !number.chars().all(is_allowed_number_char) {
        return false;
    }
    let Ok(value) = number.parse::<f32>() else {
        return false;
    };
    let factor = match unit {
        "" => 1.0,
        "t" => 1.0,
        "s" => 20.0,
        "d" => 24_000.0,
        _ => return false,
    };
    let ticks = (value * factor + 0.5).floor();
    ticks >= minimum as f32
}
fn command_argument_error_message(
    parser: &crate::ui::domain::CommandParser,
    token: &str,
) -> String {
    match parser {
        crate::ui::domain::CommandParser::Bool => {
            if token.is_empty() {
                "Expected boolean".to_owned()
            } else {
                format!("Invalid boolean: expected 'true' or 'false' but found '{token}'")
            }
        }
        crate::ui::domain::CommandParser::Number { kind, min, max } => {
            let (label, expected, invalid) = match kind {
                crate::ui::domain::CommandNumberKind::Float => {
                    ("Float", "Expected float", "Invalid float")
                }
                crate::ui::domain::CommandNumberKind::Double => {
                    ("Double", "Expected double", "Invalid double")
                }
                crate::ui::domain::CommandNumberKind::Integer => {
                    ("Integer", "Expected integer", "Invalid integer")
                }
                crate::ui::domain::CommandNumberKind::Long => {
                    ("Long", "Expected long", "Invalid long")
                }
            };
            if token.is_empty() {
                expected.to_owned()
            } else if !token.chars().all(is_allowed_number_char) {
                format!("{invalid} '{token}'")
            } else {
                let below = match kind {
                    crate::ui::domain::CommandNumberKind::Float => {
                        number_is_below::<f32>(token, min.as_deref())
                    }
                    crate::ui::domain::CommandNumberKind::Double => {
                        number_is_below::<f64>(token, min.as_deref())
                    }
                    crate::ui::domain::CommandNumberKind::Integer => {
                        number_is_below::<i32>(token, min.as_deref())
                    }
                    crate::ui::domain::CommandNumberKind::Long => {
                        number_is_below::<i64>(token, min.as_deref())
                    }
                };
                let above = match kind {
                    crate::ui::domain::CommandNumberKind::Float => {
                        number_is_above::<f32>(token, max.as_deref())
                    }
                    crate::ui::domain::CommandNumberKind::Double => {
                        number_is_above::<f64>(token, max.as_deref())
                    }
                    crate::ui::domain::CommandNumberKind::Integer => {
                        number_is_above::<i32>(token, max.as_deref())
                    }
                    crate::ui::domain::CommandNumberKind::Long => {
                        number_is_above::<i64>(token, max.as_deref())
                    }
                };
                if below {
                    let min = min.as_deref().unwrap_or_default();
                    format!("{label} must not be less than {min}: found {token}")
                } else if above {
                    let max = max.as_deref().unwrap_or_default();
                    format!("{label} must not be more than {max}: found {token}")
                } else {
                    format!("{invalid} '{token}'")
                }
            }
        }
        crate::ui::domain::CommandParser::Time { min } => {
            if token.is_empty() {
                "Expected time".to_owned()
            } else if !time_has_valid_unit(token) {
                "Invalid unit".to_owned()
            } else {
                format!("The tick count must not be less than {min}: found {token}")
            }
        }
        crate::ui::domain::CommandParser::String { .. }
        | crate::ui::domain::CommandParser::Entity { .. }
        | crate::ui::domain::CommandParser::ScoreHolder { .. }
        | crate::ui::domain::CommandParser::Named { .. } => {
            "Incorrect argument for command".to_owned()
        }
    }
}
fn number_is_below<T>(token: &str, bound: Option<&str>) -> bool
where
    T: std::str::FromStr + PartialOrd,
{
    let Some(bound) = bound else {
        return false;
    };
    let (Ok(value), Ok(bound)) = (token.parse::<T>(), bound.parse::<T>()) else {
        return false;
    };
    value < bound
}
fn number_is_above<T>(token: &str, bound: Option<&str>) -> bool
where
    T: std::str::FromStr + PartialOrd,
{
    let Some(bound) = bound else {
        return false;
    };
    let (Ok(value), Ok(bound)) = (token.parse::<T>(), bound.parse::<T>()) else {
        return false;
    };
    value > bound
}
fn time_has_valid_unit(token: &str) -> bool {
    let split = token
        .char_indices()
        .find(|(_, character)| !is_allowed_number_char(*character))
        .map(|(index, _)| index)
        .unwrap_or(token.len());
    matches!(token.split_at(split).1, "" | "t" | "s" | "d")
}
fn command_token_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut cursor = 0usize;
    while cursor < text.len() {
        while cursor < text.len() {
            let character = text[cursor..].chars().next().unwrap_or(' ');
            if !character.is_whitespace() {
                break;
            }
            cursor += character.len_utf8();
        }
        if cursor >= text.len() {
            break;
        }
        let start = cursor;
        let first = text[cursor..].chars().next().unwrap_or_default();
        if matches!(first, '"' | '\'') {
            let quote = first;
            cursor += quote.len_utf8();
            let mut escaped = false;
            while cursor < text.len() {
                let character = text[cursor..].chars().next().unwrap_or_default();
                cursor += character.len_utf8();
                if escaped {
                    escaped = false;
                } else if character == '\\' {
                    escaped = true;
                } else if character == quote {
                    break;
                }
            }
            while cursor < text.len() {
                let character = text[cursor..].chars().next().unwrap_or(' ');
                if character.is_whitespace() {
                    break;
                }
                cursor += character.len_utf8();
            }
        } else {
            while cursor < text.len() {
                let character = text[cursor..].chars().next().unwrap_or(' ');
                if character.is_whitespace() {
                    break;
                }
                cursor += character.len_utf8();
            }
        }
        spans.push((start, cursor));
    }
    spans
}
fn command_path(
    nodes: &[crate::ui::domain::CommandNode],
    mut current: usize,
    tokens: impl IntoIterator<Item = impl AsRef<str>>,
) -> Option<usize> {
    let tokens = tokens
        .into_iter()
        .map(|token| token.as_ref().to_owned())
        .collect::<Vec<_>>();
    let mut token_index = 0usize;
    while token_index < tokens.len() {
        let (child, _) = command_child(nodes, current, &tokens[token_index])?;
        let token_count = command_argument_token_count(nodes, child);
        if token_count > 1 {
            let end = token_index.checked_add(token_count)?;
            if end > tokens.len() {
                return None;
            }
            let combined = tokens[token_index..end].join(" ");
            if !command_argument_accepts_text(nodes, child, &combined) {
                return None;
            }
            token_index = end;
        } else {
            token_index += 1;
        }
        current = child;
        if is_greedy_argument(nodes, current) {
            break;
        }
    }
    Some(current)
}
fn command_argument_token_count(
    nodes: &[crate::ui::domain::CommandNode],
    node_index: usize,
) -> usize {
    let Some(node) = nodes.get(node_index) else {
        return 1;
    };
    match &node.node_type {
        crate::ui::domain::CommandNodeType::Argument {
            parser_info: crate::ui::domain::CommandParser::Named { kind, .. },
            ..
        } => match kind.split('{').next().unwrap_or(kind).trim() {
            "Vec2" | "ColumnPos" | "Rotation" => 2,
            "Vec3" | "BlockPos" => 3,
            _ => 1,
        },
        _ => 1,
    }
}
fn command_argument_accepts_text(
    nodes: &[crate::ui::domain::CommandNode],
    node_index: usize,
    text: &str,
) -> bool {
    let Some(node) = nodes.get(node_index) else {
        return false;
    };
    let crate::ui::domain::CommandNodeType::Argument { parser_info, .. } = &node.node_type else {
        return true;
    };
    if let crate::ui::domain::CommandParser::Named { kind, .. } = parser_info {
        let kind = kind.split('{').next().unwrap_or(kind).trim();
        if matches!(kind, "Vec2" | "ColumnPos" | "Rotation") {
            return valid_coordinate_argument(text, 2, false);
        }
        if matches!(kind, "Vec3" | "BlockPos") {
            return valid_coordinate_argument(text, 3, kind == "BlockPos");
        }
    }
    command_argument_accepts_token(parser_info, text)
}
fn valid_coordinate_argument(text: &str, count: usize, integer: bool) -> bool {
    let components = text.split_whitespace().collect::<Vec<_>>();
    valid_coordinate_components(&components, count, integer, true)
}
fn valid_coordinate_prefix(text: &str, count: usize, integer: bool) -> bool {
    let components = text.split_whitespace().collect::<Vec<_>>();
    valid_coordinate_components(&components, count, integer, false)
}
fn valid_coordinate_components(
    components: &[&str],
    count: usize,
    integer: bool,
    require_complete: bool,
) -> bool {
    if components.is_empty()
        || components.len() > count
        || (require_complete && components.len() != count)
    {
        return false;
    }
    let local = components[0].starts_with('^');
    for component in components {
        let value = if local {
            component.strip_prefix('^')
        } else {
            component.strip_prefix('~').or(Some(component))
        };
        let Some(value) = value else {
            return false;
        };
        if local && !component.starts_with('^') {
            return false;
        }
        if !value.is_empty()
            && (!value.chars().all(is_allowed_number_char)
                || if integer && !local && !component.starts_with('~') {
                    value.parse::<i32>().is_err()
                } else {
                    value.parse::<f64>().is_err()
                })
        {
            return false;
        }
    }
    true
}
fn is_greedy_argument(nodes: &[crate::ui::domain::CommandNode], node_index: usize) -> bool {
    nodes
        .get(node_index)
        .is_some_and(|node| match &node.node_type {
            crate::ui::domain::CommandNodeType::Argument {
                parser_info:
                    crate::ui::domain::CommandParser::String {
                        kind: crate::ui::domain::CommandStringKind::GreedyPhrase,
                    },
                ..
            } => true,
            crate::ui::domain::CommandNodeType::Argument {
                parser_info: crate::ui::domain::CommandParser::Named { kind, .. },
                ..
            } => {
                matches!(
                    kind.split('{').next().unwrap_or(kind).trim(),
                    "Message"
                        | "Component"
                        | "Style"
                        | "FormattedText"
                        | "NbtCompoundTag"
                        | "NbtTag"
                        | "NbtPath"
                        | "BlockState"
                        | "BlockPredicate"
                        | "ItemStack"
                        | "ItemPredicate"
                        | "Particle"
                )
            }
            _ => false,
        })
}
fn command_context_node(
    nodes: &[crate::ui::domain::CommandNode],
    root_index: usize,
    body: &str,
    last_word_start: usize,
) -> Option<usize> {
    let spans = command_token_spans(body);
    if spans.is_empty() {
        return Some(root_index);
    }
    let all_tokens = spans
        .iter()
        .map(|(start, end)| &body[*start..*end])
        .collect::<Vec<_>>();
    if body
        .chars()
        .last()
        .is_some_and(|character| character.is_whitespace())
    {
        return command_path(nodes, root_index, all_tokens);
    }
    if let Some(current) = command_path(nodes, root_index, all_tokens.iter().copied()) {
        return Some(current);
    }
    if let Some(current) =
        command_path_for_suggestions(nodes, root_index, all_tokens.iter().copied())
    {
        return Some(current);
    }
    let prefix = body[..last_word_start.min(body.len())]
        .split_whitespace()
        .collect::<Vec<_>>();
    command_path_for_suggestions(nodes, root_index, prefix)
}
fn command_path_for_suggestions(
    nodes: &[crate::ui::domain::CommandNode],
    mut current: usize,
    tokens: impl IntoIterator<Item = impl AsRef<str>>,
) -> Option<usize> {
    let tokens = tokens
        .into_iter()
        .map(|token| token.as_ref().to_owned())
        .collect::<Vec<_>>();
    let mut token_index = 0usize;
    while token_index < tokens.len() {
        let (child, _) = command_child(nodes, current, &tokens[token_index])?;
        let token_count = command_argument_token_count(nodes, child);
        if token_count > 1 {
            let remaining = tokens.len().saturating_sub(token_index);
            if remaining < token_count {
                let combined = tokens[token_index..].join(" ");
                if command_argument_accepts_text(nodes, child, &combined)
                    || is_coordinate_argument(nodes, child)
                        && valid_coordinate_prefix(
                            &combined,
                            token_count,
                            is_block_pos_argument(nodes, child),
                        )
                {
                    return Some(child);
                }
                return None;
            }
            let end = token_index.checked_add(token_count)?;
            let combined = tokens[token_index..end].join(" ");
            if !command_argument_accepts_text(nodes, child, &combined) {
                return None;
            }
            token_index = end;
        } else {
            token_index += 1;
        }
        current = child;
        if is_greedy_argument(nodes, current) {
            break;
        }
    }
    Some(current)
}
fn is_coordinate_argument(nodes: &[crate::ui::domain::CommandNode], node_index: usize) -> bool {
    matches!(command_argument_token_count(nodes, node_index), 2 | 3)
}
fn is_block_pos_argument(nodes: &[crate::ui::domain::CommandNode], node_index: usize) -> bool {
    nodes.get(node_index).is_some_and(|node| {
        matches!(
            & node.node_type, crate ::ui::domain::CommandNodeType::Argument {
            parser_info : crate ::ui::domain::CommandParser::Named { kind, .. }, .. }
            if kind.split('{').next().unwrap_or(kind).trim() == "BlockPos"
        )
    })
}
fn command_child_error(
    nodes: &[crate::ui::domain::CommandNode],
    mut parent_index: usize,
    token: &str,
) -> String {
    for _ in 0..nodes.len().max(1) {
        let Some(parent) = nodes.get(parent_index) else {
            return "Incorrect argument for command".to_owned();
        };
        let mut parser_error = None;
        for child_index in command_children(nodes, parent_index) {
            let Some(child) = nodes.get(child_index as usize) else {
                continue;
            };
            match &child.node_type {
                crate::ui::domain::CommandNodeType::Literal { name } if name == token => {
                    return String::new();
                }
                crate::ui::domain::CommandNodeType::Argument { parser_info, .. } => {
                    if command_argument_accepts_token(parser_info, token) {
                        return String::new();
                    }
                    parser_error
                        .get_or_insert_with(|| command_argument_error_message(parser_info, token));
                }
                _ => {}
            }
        }
        if let Some(parser_error) = parser_error {
            return parser_error;
        }
        let Some(redirect_index) = parent.redirect_node else {
            return "Incorrect argument for command".to_owned();
        };
        parent_index = redirect_index as usize;
    }
    "Incorrect argument for command".to_owned()
}
pub fn command_parse_error_for_input(
    text: &str,
    cursor: usize,
    root_index: u32,
    nodes: &[crate::ui::domain::CommandNode],
) -> Option<String> {
    let cursor = cursor.min(text.chars().count());
    if cursor != text.chars().count() || !text.starts_with('/') || nodes.is_empty() {
        return None;
    }
    let body = &text[1..];
    if body.is_empty()
        || body
            .chars()
            .last()
            .is_some_and(|character| character.is_whitespace())
    {
        return None;
    }
    let spans = command_token_spans(body);
    let all_tokens = spans
        .iter()
        .map(|(start, end)| &body[*start..*end])
        .collect::<Vec<_>>();
    if command_path(nodes, root_index as usize, all_tokens.iter().copied()).is_some() {
        return None;
    }
    let Some((last_start, last_end)) = spans.last().copied() else {
        return None;
    };
    let prefix = spans[..spans.len().saturating_sub(1)]
        .iter()
        .map(|(start, end)| &body[*start..*end])
        .collect::<Vec<_>>();
    let Some(current) = command_path(nodes, root_index as usize, prefix.iter().copied()) else {
        let position = 1 + body[..last_start].chars().count();
        return Some(format_exception_line(
            if prefix.is_empty() {
                "Unknown or incomplete command. See below for error"
            } else {
                "Incorrect argument for command"
            },
            text,
            position,
        ));
    };
    let partial = &body[last_start..last_end];
    if command_child(nodes, current, partial).is_some() {
        return None;
    }
    let position = 1 + body[..last_start].chars().count();
    Some(format_exception_line(
        &command_child_error(nodes, current, partial),
        text,
        position,
    ))
}
fn format_exception_line(message: &str, input: &str, position: usize) -> String {
    let position = position.min(input.chars().count());
    let context_start = position.saturating_sub(10);
    let context = if context_start > 0 {
        format!(
            "...{}<--[HERE]",
            input
                .chars()
                .skip(context_start)
                .take(position - context_start)
                .collect::<String>()
        )
    } else {
        format!(
            "{}<--[HERE]",
            input.chars().take(position).collect::<String>()
        )
    };
    format!("{message} at position {position}: {context}")
}
fn command_child(
    nodes: &[crate::ui::domain::CommandNode],
    mut parent_index: usize,
    token: &str,
) -> Option<(usize, bool)> {
    for _ in 0..nodes.len().max(1) {
        let parent = nodes.get(parent_index)?;
        let mut argument_child = None;
        for child_index in command_children(nodes, parent_index) {
            let child_index = child_index as usize;
            let child = nodes.get(child_index)?;
            match &child.node_type {
                crate::ui::domain::CommandNodeType::Literal { name } if name == token => {
                    return Some((child_index, false));
                }
                crate::ui::domain::CommandNodeType::Argument { parser_info, .. }
                    if command_argument_accepts_token(parser_info, token) =>
                {
                    argument_child.get_or_insert((child_index, true));
                }
                _ => {}
            }
        }
        if let Some(argument_child) = argument_child {
            return Some(argument_child);
        }
        let Some(redirect_index) = parent.redirect_node else {
            return None;
        };
        parent_index = redirect_index as usize;
    }
    None
}
fn command_children(nodes: &[crate::ui::domain::CommandNode], mut parent_index: usize) -> Vec<u32> {
    for _ in 0..nodes.len().max(1) {
        let Some(parent) = nodes.get(parent_index) else {
            return Vec::new();
        };
        if !parent.children.is_empty() {
            return parent.children.clone();
        }
        let Some(redirect_index) = parent.redirect_node else {
            return Vec::new();
        };
        parent_index = redirect_index as usize;
    }
    Vec::new()
}
#[allow(dead_code)]
pub fn build_chat_suggestions(text: &str, cursor: usize) -> Vec<ChatSuggestion> {
    build_chat_suggestions_with_commands(text, cursor, &[])
}
pub fn build_chat_suggestions_with_commands(
    text: &str,
    cursor: usize,
    server_commands: &[String],
) -> Vec<ChatSuggestion> {
    let cursor = cursor.min(text.chars().count());
    let cursor_byte = byte_index(text, cursor);
    let before_cursor = &text[..cursor_byte];
    if !before_cursor.starts_with('/') {
        return Vec::new();
    }
    let command_body = &before_cursor[1..];
    let command_body_starts_at = 1;
    let word_start_in_body = get_last_word_index(command_body);
    let word_start_byte = command_body_starts_at + word_start_in_body;
    let word_start = before_cursor[..word_start_byte].chars().count();
    let word_prefix = &before_cursor[word_start_byte..];
    let tokens: Vec<&str> = command_body.split_whitespace().collect();
    if tokens.is_empty() || (tokens.len() == 1 && word_start_in_body == 0) {
        return if server_commands.is_empty() {
            suggest_candidates(
                command_names().iter().copied(),
                word_start,
                cursor,
                word_prefix,
            )
        } else {
            suggest_candidates(
                server_commands.iter().map(String::as_str),
                word_start,
                cursor,
                word_prefix,
            )
        };
    }
    let command = tokens.first().copied().unwrap_or("").to_ascii_lowercase();
    match command.as_str() {
        "advancement" => suggest_candidates(
            ["grant", "revoke"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "attribute" => suggest_candidates(
            ["get", "base", "modifier"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "bossbar" => suggest_candidates(
            ["add", "get", "list", "remove", "set"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "clear" | "give" | "item" => item_suggestions(word_start, cursor, word_prefix),
        "clone" => suggest_coordinates(word_start, cursor, word_prefix),
        "damage" => suggest_entity_selectors(word_start, cursor, word_prefix),
        "datapack" => suggest_candidates(
            ["disable", "enable", "list"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "debug" => suggest_candidates(
            ["start", "stop", "function", "configmobspawning"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "defaultgamemode" | "gamemode" => suggest_candidates(
            ["survival", "creative", "adventure", "spectator"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "difficulty" => suggest_candidates(
            ["peaceful", "easy", "normal", "hard"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "effect" => suggest_candidates(
            ["clear", "give"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "execute" => suggest_candidates(
            [
                "align",
                "anchored",
                "as",
                "at",
                "facing",
                "if",
                "in",
                "on",
                "positioned",
                "rotated",
                "run",
                "store",
                "summon",
                "unless",
            ]
            .into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "experience" | "xp" => suggest_candidates(
            ["add", "query", "set"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "fill" | "fillbiome" | "setblock" => block_suggestions(word_start, cursor, word_prefix),
        "forceload" => suggest_candidates(
            ["add", "query", "remove"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "function" => suggest_candidates(["*"].into_iter(), word_start, cursor, word_prefix),
        "gamerule" => gamerule_suggestions(word_start, cursor, word_prefix),
        "help" => {
            if server_commands.is_empty() {
                suggest_candidates(
                    command_names().iter().copied(),
                    word_start,
                    cursor,
                    word_prefix,
                )
            } else {
                suggest_candidates(
                    server_commands.iter().map(String::as_str),
                    word_start,
                    cursor,
                    word_prefix,
                )
            }
        }
        "kill" | "kick" | "list" | "msg" | "tell" | "teammsg" | "tm" | "w" => {
            suggest_entity_selectors(word_start, cursor, word_prefix)
        }
        "locate" => suggest_candidates(
            ["biome", "poi", "structure"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "loot" => suggest_candidates(
            ["fish", "kill", "loot", "mine", "spawn"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "particle" => particle_suggestions(word_start, cursor, word_prefix),
        "place" => suggest_candidates(
            ["feature", "jigsaw", "structure", "template"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "playsound" | "stopsound" => sound_suggestions(word_start, cursor, word_prefix),
        "recipe" => suggest_candidates(
            ["give", "take"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "ride" => suggest_candidates(
            ["mount", "dismount"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "scoreboard" => suggest_candidates(
            ["objectives", "players"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "schedule" => suggest_candidates(
            ["clear", "function"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "spreadplayers" | "summon" | "tp" | "teleport" => {
            suggest_coordinates(word_start, cursor, word_prefix)
        }
        "tag" => suggest_candidates(
            ["add", "list", "remove"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "team" => suggest_candidates(
            ["add", "empty", "join", "leave", "list", "modify", "remove"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "time" if tokens.get(1) == Some(&"set") => suggest_candidates(
            ["day", "noon", "night", "midnight"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "time" => suggest_candidates(
            ["add", "query", "set"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "title" => suggest_candidates(
            ["actionbar", "clear", "reset", "subtitle", "times", "title"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "weather" => suggest_candidates(
            ["clear", "rain", "thunder"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        "worldborder" => suggest_candidates(
            ["add", "center", "damage", "get", "set", "warning"].into_iter(),
            word_start,
            cursor,
            word_prefix,
        ),
        _ => Vec::new(),
    }
}
pub fn build_custom_chat_completions(
    text: &str,
    cursor: usize,
    custom_completions: &[String],
) -> Vec<ChatSuggestion> {
    let cursor = cursor.min(text.chars().count());
    let cursor_byte = byte_index(text, cursor);
    let before_cursor = &text[..cursor_byte];
    if before_cursor.starts_with('/') || before_cursor.trim().is_empty() {
        return Vec::new();
    }
    let word_start_byte = get_last_word_index(before_cursor);
    let word_start = before_cursor[..word_start_byte].chars().count();
    let word_prefix = &before_cursor[word_start_byte..];
    suggest_candidates(
        custom_completions.iter().map(String::as_str),
        word_start,
        cursor,
        word_prefix,
    )
}
pub fn command_names() -> &'static [&'static str] {
    &["gamemode", "give", "help", "kill", "time", "tp", "weather"]
}
fn item_suggestions(word_start: usize, cursor: usize, word_prefix: &str) -> Vec<ChatSuggestion> {
    let registry = el_core::registry::BuiltInRegistries::items();
    let mut candidates = Vec::new();
    for id in 0..registry.size() {
        let item = registry.get_by_id(id);
        let full = item.location.to_string();
        candidates.push(full);
        if candidates.len() >= 512 {
            break;
        }
    }
    suggest_candidates(
        candidates.iter().map(String::as_str),
        word_start,
        cursor,
        word_prefix,
    )
}
fn block_suggestions(word_start: usize, cursor: usize, word_prefix: &str) -> Vec<ChatSuggestion> {
    let registry = el_core::registry::BuiltInRegistries::blocks();
    let mut candidates = Vec::new();
    for id in 0..registry.size() {
        candidates.push(registry.get_by_id(id).location.to_string());
        if candidates.len() >= 512 {
            break;
        }
    }
    suggest_candidates(
        candidates.iter().map(String::as_str),
        word_start,
        cursor,
        word_prefix,
    )
}
fn particle_suggestions(
    word_start: usize,
    cursor: usize,
    word_prefix: &str,
) -> Vec<ChatSuggestion> {
    suggest_candidates(
        [
            "minecraft:block",
            "minecraft:cloud",
            "minecraft:crit",
            "minecraft:dust",
            "minecraft:flame",
            "minecraft:happy_villager",
            "minecraft:poof",
            "minecraft:smoke",
            "minecraft:splash",
        ]
        .into_iter(),
        word_start,
        cursor,
        word_prefix,
    )
}
fn sound_suggestions(word_start: usize, cursor: usize, word_prefix: &str) -> Vec<ChatSuggestion> {
    suggest_candidates(
        [
            "minecraft:block.grass.break",
            "minecraft:block.grass.place",
            "minecraft:block.stone.break",
            "minecraft:block.stone.place",
            "minecraft:entity.player.hurt",
            "minecraft:entity.player.levelup",
            "minecraft:entity.zombie.ambient",
        ]
        .into_iter(),
        word_start,
        cursor,
        word_prefix,
    )
}
fn gamerule_suggestions(
    word_start: usize,
    cursor: usize,
    word_prefix: &str,
) -> Vec<ChatSuggestion> {
    suggest_candidates(
        [
            "announceAdvancements",
            "commandBlockOutput",
            "disableElytraMovementCheck",
            "disableRaids",
            "doDaylightCycle",
            "doEntityDrops",
            "doFireTick",
            "doImmediateRespawn",
            "doInsomnia",
            "doMobLoot",
            "doMobSpawning",
            "doPatrolSpawning",
            "doTileDrops",
            "doTraderSpawning",
            "doVinesSpread",
            "doWeatherCycle",
            "keepInventory",
            "logAdminCommands",
            "mobGriefing",
            "naturalRegeneration",
            "reducedDebugInfo",
            "sendCommandFeedback",
            "showDeathMessages",
            "spectatorsGenerateChunks",
        ]
        .into_iter(),
        word_start,
        cursor,
        word_prefix,
    )
}
fn suggest_coordinates(word_start: usize, cursor: usize, word_prefix: &str) -> Vec<ChatSuggestion> {
    suggest_candidates(
        ["~", "~ ~ ~", "^ ^ ^", "0 0 0"].into_iter(),
        word_start,
        cursor,
        word_prefix,
    )
}
fn suggest_entity_selectors(
    word_start: usize,
    cursor: usize,
    word_prefix: &str,
) -> Vec<ChatSuggestion> {
    suggest_candidates(
        ["@p", "@a", "@r", "@s", "@e"].into_iter(),
        word_start,
        cursor,
        word_prefix,
    )
}
fn suggest_candidates<'a>(
    candidates: impl Iterator<Item = &'a str>,
    range_start: usize,
    range_end: usize,
    prefix: &str,
) -> Vec<ChatSuggestion> {
    let mut unique = BTreeSet::new();
    let mut suggestions = Vec::new();
    for candidate in candidates {
        if !matches_vanilla_substr(prefix, candidate) || !unique.insert(candidate.to_string()) {
            continue;
        }
        suggestions.push(ChatSuggestion::new(
            candidate.to_string(),
            candidate.to_string(),
            range_start,
            range_end,
        ));
        if suggestions.len() >= 128 {
            break;
        }
    }
    sort_like_vanilla(&mut suggestions, prefix);
    suggestions
}
fn matches_vanilla_substr(prefix: &str, candidate: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    let prefix = prefix.to_lowercase();
    let candidate = candidate.to_lowercase();
    candidate.starts_with(&prefix)
        || candidate
            .split_once(':')
            .is_some_and(|(_, path)| path.starts_with(&prefix))
}
fn sort_like_vanilla(suggestions: &mut Vec<ChatSuggestion>, last_word: &str) {
    let last_word = last_word.to_lowercase();
    let minecraft_prefixed = format!("minecraft:{last_word}");
    let mut exact_prefix = Vec::new();
    let mut partial = Vec::new();
    for suggestion in suggestions.drain(..) {
        let text = suggestion.replacement.to_lowercase();
        if text.starts_with(&last_word) || text.starts_with(&minecraft_prefixed) {
            exact_prefix.push(suggestion);
        } else {
            partial.push(suggestion);
        }
    }
    exact_prefix.sort_by(|a, b| a.display.to_lowercase().cmp(&b.display.to_lowercase()));
    partial.sort_by(|a, b| a.display.to_lowercase().cmp(&b.display.to_lowercase()));
    exact_prefix.append(&mut partial);
    *suggestions = exact_prefix;
}
pub fn last_word_index(text: &str) -> usize {
    text.char_indices()
        .filter(|(_, ch)| ch.is_whitespace())
        .map(|(idx, ch)| idx + ch.len_utf8())
        .last()
        .unwrap_or(0)
}
fn get_last_word_index(text: &str) -> usize {
    last_word_index(text)
}
fn byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len())
}
