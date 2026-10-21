use crate::gameplay::game_mode::GameType;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScoreNumberFormat {
    Blank,
    Styled {
        color: Option<i32>,
        style: Option<serde_json::Value>,
    },
    Fixed {
        text: String,
    },
}
pub const DISPLAY_SLOT_LIST: usize = 0;
pub const DISPLAY_SLOT_SIDEBAR: usize = 1;
pub const DISPLAY_SLOT_BELOW_NAME: usize = 2;
pub const DISPLAY_SLOT_TEAM_BASE: usize = 3;
pub const DISPLAY_SLOT_COUNT: usize = 19;
pub const TAB_MAX_ROWS_PER_COL: i32 = 20;
pub const SIDEBAR_SCORE_COLOR: u32 = 0xFF5555;
pub const PLAYER_LIST_SCORE_COLOR: u32 = 0xFFFF55;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenderType {
    Integer,
    Hearts,
}
impl RenderType {
    fn from_u8(value: u8) -> Self {
        match value {
            1 => Self::Hearts,
            _ => Self::Integer,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Objective {
    pub name: String,
    pub display_name: String,
    pub render_type: RenderType,
    pub number_format: Option<ScoreNumberFormat>,
}
#[derive(Clone, Debug)]
pub struct Score {
    pub value: i32,
    pub display: Option<String>,
    pub number_format: Option<ScoreNumberFormat>,
}
#[derive(Clone, Debug)]
pub struct PlayerTeam {
    pub name: String,
    pub display_name: String,
    pub prefix: String,
    pub suffix: String,
    pub color_id: i32,
    pub nametag_visibility: String,
    pub collision_rule: String,
    pub members: HashSet<String>,
}
#[derive(Clone, Debug)]
pub struct SidebarEntry {
    pub name: String,
    pub score: String,
}
#[derive(Clone, Debug)]
pub struct PlayerInfoState {
    pub uuid: Uuid,
    pub name: String,
    pub texture_url: Option<String>,
    pub model: Option<String>,
    pub game_mode: GameType,
    pub latency: i32,
    pub display_name: Option<String>,
    pub listed: bool,
    pub list_order: i32,
    pub show_hat: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Scoreboard {
    objectives: HashMap<String, Objective>,
    scores: HashMap<String, HashMap<String, Score>>,
    display_slots: [Option<String>; DISPLAY_SLOT_COUNT],
    teams: HashMap<String, PlayerTeam>,
    player_teams: HashMap<String, String>,
}
impl Scoreboard {
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn set_objective(
        &mut self,
        name: String,
        title: String,
        render_type: u8,
        number_format: Option<ScoreNumberFormat>,
        method: u8,
    ) {
        match method {
            1 => {
                self.objectives.remove(&name);
                self.scores.remove(&name);
                for slot in self.display_slots.iter_mut() {
                    if slot.as_deref() == Some(name.as_str()) {
                        *slot = None;
                    }
                }
            }
            _ => {
                self.objectives.insert(
                    name.clone(),
                    Objective {
                        name,
                        display_name: title,
                        render_type: RenderType::from_u8(render_type),
                        number_format,
                    },
                );
            }
        }
    }
    pub fn set_score(
        &mut self,
        owner: String,
        objective: String,
        value: i32,
        display: Option<String>,
        number_format: Option<ScoreNumberFormat>,
    ) {
        self.scores.entry(objective).or_default().insert(
            owner,
            Score {
                value,
                display,
                number_format,
            },
        );
    }
    pub fn reset_score(&mut self, owner: &str, objective: Option<&str>) {
        match objective {
            Some(objective) => {
                if let Some(scores) = self.scores.get_mut(objective) {
                    scores.remove(owner);
                }
            }
            None => {
                for scores in self.scores.values_mut() {
                    scores.remove(owner);
                }
            }
        }
    }
    pub fn set_display_objective(&mut self, slot: u8, objective_name: String) {
        let slot = slot as usize;
        if slot >= DISPLAY_SLOT_COUNT {
            return;
        }
        self.display_slots[slot] = if objective_name.is_empty() {
            None
        } else {
            Some(objective_name)
        };
    }
    pub fn get_display_objective(&self, slot: usize) -> Option<&Objective> {
        self.display_slots
            .get(slot)
            .and_then(|name| name.as_ref())
            .and_then(|name| self.objectives.get(name))
    }
    pub fn get_players_team(&self, holder: &str) -> Option<&PlayerTeam> {
        self.player_teams
            .get(holder)
            .and_then(|team| self.teams.get(team))
    }
    pub fn below_name_display(&self, holder: &str) -> Option<String> {
        let objective = self.get_display_objective(DISPLAY_SLOT_BELOW_NAME)?;
        let score = self.scores.get(&objective.name)?.get(holder)?;
        let score_text = match score
            .number_format
            .as_ref()
            .or(objective.number_format.as_ref())
        {
            None => score.value.to_string(),
            Some(ScoreNumberFormat::Blank) => String::new(),
            Some(ScoreNumberFormat::Fixed { text }) => text.clone(),
            Some(ScoreNumberFormat::Styled { color, style }) => {
                styled_score_value(score.value, *color, style.as_ref())
            }
        };
        Some(format!("{score_text} {0}", objective.display_name))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn set_team(
        &mut self,
        name: String,
        op: u8,
        display_name: Option<String>,
        prefix: Option<String>,
        suffix: Option<String>,
        color_id: i32,
        nametag_visibility: Option<String>,
        collision_rule: Option<String>,
        members: Option<Vec<String>>,
    ) {
        match op {
            0 => {
                let mut team = PlayerTeam {
                    name: name.clone(),
                    display_name: display_name.unwrap_or_default(),
                    prefix: prefix.unwrap_or_default(),
                    suffix: suffix.unwrap_or_default(),
                    color_id,
                    nametag_visibility: nametag_visibility.unwrap_or_else(|| "always".to_string()),
                    collision_rule: collision_rule.unwrap_or_else(|| "always".to_string()),
                    members: HashSet::new(),
                };
                if let Some(members) = members {
                    for member in members {
                        self.player_teams.insert(member.clone(), name.clone());
                        team.members.insert(member);
                    }
                }
                self.teams.insert(name, team);
            }
            1 => {
                if let Some(team) = self.teams.remove(&name) {
                    for member in team.members {
                        if self.player_teams.get(&member) == Some(&name) {
                            self.player_teams.remove(&member);
                        }
                    }
                }
            }
            2 => {
                if let Some(team) = self.teams.get_mut(&name) {
                    if let Some(display_name) = display_name {
                        team.display_name = display_name;
                    }
                    if let Some(prefix) = prefix {
                        team.prefix = prefix;
                    }
                    if let Some(suffix) = suffix {
                        team.suffix = suffix;
                    }
                    team.color_id = color_id;
                    if let Some(visibility) = nametag_visibility {
                        team.nametag_visibility = visibility;
                    }
                    if let Some(rule) = collision_rule {
                        team.collision_rule = rule;
                    }
                }
            }
            3 => {
                if self.teams.contains_key(&name) {
                    if let Some(members) = members {
                        for member in members {
                            self.player_teams.insert(member.clone(), name.clone());
                            if let Some(team) = self.teams.get_mut(&name) {
                                team.members.insert(member);
                            }
                        }
                    }
                }
            }
            4 => {
                if let Some(members) = members {
                    for member in members {
                        if let Some(team) = self.teams.get_mut(&name) {
                            team.members.remove(&member);
                        }
                        if self.player_teams.get(&member) == Some(&name) {
                            self.player_teams.remove(&member);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    pub fn sidebar_entries(&self, objective: &Objective) -> Vec<SidebarEntry> {
        let Some(scores) = self.scores.get(&objective.name) else {
            return Vec::new();
        };
        let mut sorted: Vec<(&String, &Score)> = scores
            .iter()
            .filter(|(owner, _)| !owner.starts_with('#'))
            .collect();
        sorted.sort_by(|(owner_a, score_a), (owner_b, score_b)| {
            score_b
                .value
                .cmp(&score_a.value)
                .then_with(|| owner_a.to_lowercase().cmp(&owner_b.to_lowercase()))
        });
        sorted.truncate(15);
        sorted
            .into_iter()
            .map(|(owner, score)| {
                let owner_name = score.display.clone().unwrap_or_else(|| owner.clone());
                let name = format_name_for_team(self.get_players_team(owner), &owner_name);
                let score_text = format_score_value(
                    score.value,
                    score.number_format.as_ref(),
                    objective.number_format.as_ref(),
                    SIDEBAR_SCORE_COLOR,
                );
                SidebarEntry {
                    name,
                    score: score_text,
                }
            })
            .collect()
    }
    pub fn player_score(&self, holder: &str, objective: &str) -> Option<&Score> {
        self.scores.get(objective).and_then(|s| s.get(holder))
    }
}
pub fn ansi_rgb(rgb: u32) -> String {
    format!(
        "\u{1b}[38;2;{};{};{}m",
        (rgb >> 16) & 0xFF,
        (rgb >> 8) & 0xFF,
        rgb & 0xFF
    )
}
const ANSI_RESET: &str = "\u{1b}[m";
const ANSI_ITALIC: &str = "\u{1b}[3m";
pub fn chat_color_rgb(id: i32) -> Option<u32> {
    Some(match id {
        0 => 0x000000,
        1 => 0x0000AA,
        2 => 0x00AA00,
        3 => 0x00AAAA,
        4 => 0xAA0000,
        5 => 0xAA00AA,
        6 => 0xFFAA00,
        7 => 0xAAAAAA,
        8 => 0x555555,
        9 => 0x5555FF,
        10 => 0x55FF55,
        11 => 0x55FFFF,
        12 => 0xFF5555,
        13 => 0xFF55FF,
        14 => 0xFFFF55,
        15 => 0xFFFFFF,
        _ => return None,
    })
}
pub fn format_name_for_team(team: Option<&PlayerTeam>, name_ansi: &str) -> String {
    match team {
        None => name_ansi.to_string(),
        Some(team) => {
            let mut out = String::new();
            if let Some(rgb) = chat_color_rgb(team.color_id) {
                let parent_color = ansi_rgb(rgb);
                out.push_str(&team_colored_name_fragment(&team.prefix, &parent_color));
                out.push_str(&team_colored_name_fragment(name_ansi, &parent_color));
                out.push_str(&team_colored_name_fragment(&team.suffix, &parent_color));
                out.push_str(ANSI_RESET);
            } else {
                out.push_str(&team.prefix);
                out.push_str(name_ansi);
                out.push_str(&team.suffix);
            }
            out
        }
    }
}
fn team_colored_name_fragment(fragment: &str, parent_color: &str) -> String {
    let mut out = String::with_capacity(parent_color.len() + fragment.len() + 16);
    out.push_str(parent_color);
    let bytes = fragment.as_bytes();
    let mut cursor = 0;
    while cursor < bytes.len() {
        if bytes[cursor] == 0x1B && bytes.get(cursor + 1) == Some(&b'[') {
            let parameters_start = cursor + 2;
            let Some(relative_end) = fragment[parameters_start..].find('m') else {
                out.push_str(&fragment[cursor..]);
                return out;
            };
            let end = parameters_start + relative_end;
            out.push_str(&fragment[cursor..=end]);
            if sgr_returns_to_parent_color(&fragment[parameters_start..end]) {
                out.push_str(parent_color);
            }
            cursor = end + 1;
            continue;
        }
        if bytes[cursor] == 0xC2
            && bytes.get(cursor + 1) == Some(&0xA7)
            && let Some(&code) = bytes.get(cursor + 2)
        {
            out.push_str(&fragment[cursor..cursor + 3]);
            if matches!(code, b'r' | b'R') {
                out.push_str(parent_color);
            }
            cursor += 3;
            continue;
        }
        let next = fragment[cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| cursor + offset)
            .unwrap_or(bytes.len());
        out.push_str(&fragment[cursor..next]);
        cursor = next;
    }
    out
}
fn sgr_returns_to_parent_color(parameters: &str) -> bool {
    if parameters.is_empty() {
        return true;
    }
    let values = parameters
        .split(';')
        .map(|value| value.parse::<u16>().ok())
        .collect::<Vec<_>>();
    let mut parent_required = false;
    let mut index = 0;
    while index < values.len() {
        match values[index] {
            Some(0 | 39) => parent_required = true,
            Some(30..=37 | 90..=97) => parent_required = false,
            Some(38) => match values.get(index + 1).copied().flatten() {
                Some(2) if index + 4 < values.len() => {
                    parent_required = false;
                    index += 4;
                }
                Some(5) if index + 2 < values.len() => {
                    parent_required = false;
                    index += 2;
                }
                Some(_) | None => parent_required = false,
            },
            _ => {}
        }
        index += 1;
    }
    parent_required
}
pub fn format_score_value(
    value: i32,
    score_format: Option<&ScoreNumberFormat>,
    objective_format: Option<&ScoreNumberFormat>,
    default_color: u32,
) -> String {
    match score_format.or(objective_format) {
        None => format!("{}{}{}", ansi_rgb(default_color), value, ANSI_RESET),
        Some(ScoreNumberFormat::Blank) => String::new(),
        Some(ScoreNumberFormat::Fixed { text }) => text.clone(),
        Some(ScoreNumberFormat::Styled { color, style }) => {
            styled_score_value(value, *color, style.as_ref())
        }
    }
}
fn styled_score_value(value: i32, color: Option<i32>, style: Option<&serde_json::Value>) -> String {
    if let Some(style) = style.and_then(serde_json::Value::as_object) {
        let mut component = style.clone();
        component.insert("text".into(), serde_json::Value::String(value.to_string()));
        return crate::ui::gui::screen::container::styled_text_component(
            &serde_json::Value::Object(component),
        );
    }
    match color {
        Some(color) => format!("{}{}{}", ansi_rgb(color as u32), value, ANSI_RESET),
        None => value.to_string(),
    }
}
pub fn tab_list_name_for_display(scoreboard: &Scoreboard, info: &PlayerInfoState) -> String {
    let base = match &info.display_name {
        Some(display_name) => display_name.clone(),
        None => format_name_for_team(scoreboard.get_players_team(&info.name), &info.name),
    };
    if info.game_mode == GameType::Spectator {
        format!("{ANSI_ITALIC}{base}{ANSI_RESET}")
    } else {
        base
    }
}
#[allow(dead_code)]
pub fn sort_tab_list(scoreboard: &Scoreboard, players: &mut [PlayerInfoState]) {
    players.sort_by(|a, b| compare_tab_players(scoreboard, a, b));
}
pub fn sort_tab_list_refs<'a>(scoreboard: &Scoreboard, players: &mut [&'a PlayerInfoState]) {
    players.sort_by(|a, b| compare_tab_players(scoreboard, a, b));
}
fn compare_tab_players(
    scoreboard: &Scoreboard,
    a: &PlayerInfoState,
    b: &PlayerInfoState,
) -> std::cmp::Ordering {
    let order = a
        .list_order
        .wrapping_neg()
        .cmp(&b.list_order.wrapping_neg());
    let spectator = tab_spectator_rank(a).cmp(&tab_spectator_rank(b));
    let team = tab_team_name(scoreboard, a).cmp(tab_team_name(scoreboard, b));
    order
        .then(spectator)
        .then(team)
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
}
fn tab_spectator_rank(info: &PlayerInfoState) -> u8 {
    if info.game_mode == GameType::Spectator {
        1
    } else {
        0
    }
}
fn tab_team_name<'a>(scoreboard: &'a Scoreboard, info: &PlayerInfoState) -> &'a str {
    scoreboard
        .get_players_team(&info.name)
        .map(|team| team.name.as_str())
        .unwrap_or("")
}
pub fn sidebar_objective<'a>(
    scoreboard: &'a Scoreboard,
    local_player_name: &str,
) -> Option<&'a Objective> {
    if let Some(team) = scoreboard.get_players_team(local_player_name) {
        if team.color_id >= 0 {
            let slot = DISPLAY_SLOT_TEAM_BASE + team.color_id as usize;
            if let Some(objective) = scoreboard.get_display_objective(slot) {
                return Some(objective);
            }
        }
    }
    scoreboard.get_display_objective(DISPLAY_SLOT_SIDEBAR)
}
