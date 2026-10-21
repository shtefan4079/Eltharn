use crate::gameplay::game_mode::GameType;
use crate::ui::chat::chat_component::{
    draw_formatted_chat_text_ex, formatted_chat_width, split_formatted_chat_lines,
};
use crate::ui::gui::font::FontRenderer;
use crate::ui::gui::renderer::GuiRenderer;
use crate::ui::scoreboard::{
    self, PLAYER_LIST_SCORE_COLOR, PlayerInfoState, RenderType, Scoreboard,
};
use el_core::resource_location::ResourceLocation;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HudBackgroundOptions {
    pub background_for_chat_only: bool,
    pub text_background_opacity: f32,
}
impl HudBackgroundOptions {
    fn color_from_argb(&self, default_argb: u32) -> [f32; 4] {
        if self.background_for_chat_only {
            argb_to_rgba(default_argb)
        } else {
            [0.0, 0.0, 0.0, java_alpha(self.text_background_opacity)]
        }
    }
    pub(crate) fn chat_input_background(&self) -> [f32; 4] {
        self.color_from_argb(0x8000_0000)
    }
    pub(crate) fn get_background_opacity(&self, default_opacity: f32) -> f32 {
        if self.background_for_chat_only {
            default_opacity
        } else {
            self.text_background_opacity
        }
    }
    pub(crate) fn color_from_opacity(&self, default_opacity: f32) -> [f32; 4] {
        [
            0.0,
            0.0,
            0.0,
            java_alpha(self.get_background_opacity(default_opacity)),
        ]
    }
    pub(crate) fn text_with_backdrop_color(
        &self,
        default_opacity: f32,
        text_alpha: f32,
    ) -> Option<[f32; 4]> {
        let bg = self.get_background_opacity(default_opacity);
        if bg <= 0.0 {
            return None;
        }
        let bg_byte = (bg.clamp(0.0, 1.0) * 255.0).floor() as u32;
        let text_byte = (text_alpha.clamp(0.0, 1.0) * 255.0).floor() as u32;
        let alpha = (bg_byte * text_byte / 255) as f32 / 255.0;
        Some([0.0, 0.0, 0.0, alpha])
    }
}
fn java_alpha(value: f32) -> f32 {
    ((value.clamp(0.0, 1.0) * 255.0).floor() as u8) as f32 / 255.0
}
fn java_font_width(font: &FontRenderer, text: &str) -> i32 {
    formatted_chat_width(font, text, 1.0).ceil() as i32
}
fn java_tab_gui_width(logical_width: f32) -> i32 {
    logical_width.ceil() as i32
}
fn argb_to_rgba(argb: u32) -> [f32; 4] {
    [
        ((argb >> 16) & 0xFF) as f32 / 255.0,
        ((argb >> 8) & 0xFF) as f32 / 255.0,
        (argb & 0xFF) as f32 / 255.0,
        ((argb >> 24) & 0xFF) as f32 / 255.0,
    ]
}
const WHITE: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const TAB_SECTION_BG: [f32; 4] = [0.0, 0.0, 0.0, 128.0 / 255.0];
const TAB_ROW_DEFAULT_ARGB: u32 = 553_648_127;
pub fn render_scoreboard_sidebar(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    scoreboard: &Scoreboard,
    local_player_name: &str,
    screen_w: f32,
    screen_h: f32,
    opts: HudBackgroundOptions,
) {
    let Some(objective) = scoreboard::sidebar_objective(scoreboard, local_player_name) else {
        return;
    };
    let entries = scoreboard.sidebar_entries(objective);
    let spacer_width = java_font_width(font, ": ");
    let objective_name = &objective.display_name;
    let objective_name_width = java_font_width(font, objective_name);
    let mut biggest_width = objective_name_width;
    let mut row_score_widths = Vec::with_capacity(entries.len());
    for entry in &entries {
        let name_width = java_font_width(font, &entry.name);
        let score_width = java_font_width(font, &entry.score);
        let line_width = name_width
            + if score_width > 0 {
                spacer_width + score_width
            } else {
                0
            };
        biggest_width = biggest_width.max(line_width);
        row_score_widths.push(score_width);
    }
    let gui_width = java_tab_gui_width(screen_w);
    let gui_height = java_tab_gui_width(screen_h);
    let entries_count = entries.len() as i32;
    let height = entries_count * 9;
    let bottom = gui_height / 2 + height / 3;
    let left = gui_width - biggest_width - 3;
    let right = gui_width - 3 + 2;
    let background = opts.color_from_opacity(0.3);
    let header_background = opts.color_from_opacity(0.4);
    let header_y = bottom - entries_count * 9;
    renderer.draw_rect(
        (left - 2) as f32,
        (header_y - 10) as f32,
        (right - (left - 2)).max(0) as f32,
        9.0,
        header_background,
    );
    renderer.draw_rect(
        (left - 2) as f32,
        (header_y - 1) as f32,
        (right - (left - 2)).max(0) as f32,
        (bottom - (header_y - 1)).max(0) as f32,
        background,
    );
    draw_formatted_chat_text_ex(
        renderer,
        font,
        objective_name,
        (left + biggest_width / 2 - objective_name_width / 2) as f32,
        (header_y - 9) as f32,
        1.0,
        1.0,
        false,
    );
    for (i, entry) in entries.iter().enumerate() {
        let y = (bottom - (entries_count - i as i32) * 9) as f32;
        draw_formatted_chat_text_ex(renderer, font, &entry.name, left as f32, y, 1.0, 1.0, false);
        let score_width = row_score_widths[i];
        if score_width > 0 {
            draw_formatted_chat_text_ex(
                renderer,
                font,
                &entry.score,
                (right - score_width) as f32,
                y,
                1.0,
                1.0,
                false,
            );
        }
    }
}
pub fn render_tab_list(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    scoreboard: &Scoreboard,
    players: &[&PlayerInfoState],
    header: &str,
    footer: &str,
    screen_w: f32,
    opts: HudBackgroundOptions,
    health_states: &mut HashMap<Uuid, TabHealthState>,
    gui_tick: u32,
    online_mode: bool,
    level_players: &HashSet<Uuid>,
) {
    let display_objective = scoreboard.get_display_objective(scoreboard::DISPLAY_SLOT_LIST);
    let spacer_width = font.measure_width(" ", 1.0).ceil() as i32;
    let mut names = Vec::with_capacity(players.len());
    let mut score_texts = Vec::with_capacity(players.len());
    let mut score_values = Vec::with_capacity(players.len());
    let mut max_name_width = 0i32;
    let mut max_score_width = 0i32;
    for info in players {
        let name = scoreboard::tab_list_name_for_display(scoreboard, info);
        max_name_width = max_name_width.max(java_font_width(font, &name));
        let mut score_text = None;
        let mut score_value = 0;
        if let Some(objective) = display_objective {
            score_value = scoreboard
                .player_score(&info.name, &objective.name)
                .map_or(0, |score| score.value);
            if objective.render_type != RenderType::Hearts {
                if let Some(score) = scoreboard.player_score(&info.name, &objective.name) {
                    let text = scoreboard::format_score_value(
                        score.value,
                        score.number_format.as_ref(),
                        objective.number_format.as_ref(),
                        PLAYER_LIST_SCORE_COLOR,
                    );
                    let width = java_font_width(font, &text);
                    if width > 0 {
                        max_score_width = max_score_width.max(spacer_width + width);
                    }
                    score_text = Some((text, width as f32));
                }
            }
        }
        names.push(name);
        score_texts.push(score_text);
        score_values.push(score_value);
    }
    let slots = players.len() as i32;
    let player_ids = players.iter().map(|info| info.uuid).collect::<HashSet<_>>();
    health_states.retain(|id, _| player_ids.contains(id));
    let mut rows = slots;
    let mut cols = 1;
    while rows > scoreboard::TAB_MAX_ROWS_PER_COL {
        cols += 1;
        rows = (slots + cols - 1) / cols;
    }
    let rows = rows.max(0);
    let width_for_score = match display_objective {
        Some(objective) if objective.render_type == RenderType::Hearts => 90,
        Some(_) => max_score_width,
        None => 0,
    };
    let show_head = online_mode;
    let screen_w_i = java_tab_gui_width(screen_w);
    let max_name_width_i = max_name_width;
    let width_for_score_i = width_for_score;
    let head_width_i = if show_head { 9 } else { 0 };
    let per_slot_i = head_width_i + max_name_width_i + width_for_score_i + 13;
    let slot_width_i = (cols * per_slot_i).min(screen_w_i - 50) / cols;
    let slot_width = slot_width_i as f32;
    let block_i = slot_width_i * cols + (cols - 1) * 5;
    let xxo = (screen_w_i / 2 - block_i / 2) as f32;
    let mut yyo = 10.0f32;
    let mut max_line_width_i = block_i;
    let tab_text_wrap_width = (screen_w_i - 50).max(1) as f32;
    let header_lines = split_formatted_chat_lines(font, header, tab_text_wrap_width);
    let footer_lines = split_formatted_chat_lines(font, footer, tab_text_wrap_width);
    for line in header_lines.iter().chain(footer_lines.iter()) {
        let line_width_i = java_font_width(font, line);
        max_line_width_i = max_line_width_i.max(line_width_i);
    }
    let center_i = screen_w_i / 2;
    let bg_left_i = center_i - max_line_width_i / 2 - 1;
    let bg_right_i = center_i + max_line_width_i / 2 + 1;
    let bg_left = bg_left_i as f32;
    let bg_width = (bg_right_i - bg_left_i) as f32;
    if !header_lines.is_empty() {
        renderer.draw_color_rect(
            bg_left,
            yyo - 1.0,
            bg_width,
            header_lines.len() as f32 * 9.0 + 1.0,
            TAB_SECTION_BG,
        );
        for line in &header_lines {
            let line_width_i = java_font_width(font, line);
            let text_x = (center_i - line_width_i / 2) as f32;
            draw_formatted_chat_text_ex(renderer, font, line, text_x, yyo, 1.0, 1.0, true);
            yyo += 9.0;
        }
        yyo += 1.0;
    }
    renderer.draw_color_rect(
        bg_left,
        yyo - 1.0,
        bg_width,
        rows as f32 * 9.0 + 1.0,
        TAB_SECTION_BG,
    );
    let row_background = opts.color_from_argb(TAB_ROW_DEFAULT_ARGB);
    for i in 0..slots {
        let col = if rows > 0 { i / rows } else { 0 };
        let row = if rows > 0 { i % rows } else { i };
        let xxo_i = xxo.floor() as i32;
        let yyo_i = yyo.floor() as i32;
        let xo_i = xxo_i + col * slot_width_i + col * 5;
        let yo_i = yyo_i + row * 9;
        let xo = xo_i as f32;
        let yo = yo_i as f32;
        renderer.draw_color_rect(xo, yo, slot_width, 8.0, row_background);
        let index = i as usize;
        let info = &players[index];
        let name = &names[index];
        let mut name_xo_i = xo_i;
        if show_head {
            render_tablist_head(renderer, info, xo, yo, level_players);
            name_xo_i += head_width_i;
        }
        let name_xo = name_xo_i as f32;
        let name_alpha = if info.game_mode == GameType::Spectator {
            144.0 / 255.0
        } else {
            1.0
        };
        draw_formatted_chat_text_ex(renderer, font, name, name_xo, yo, name_alpha, 1.0, true);
        if let Some(objective) = display_objective {
            if info.game_mode != GameType::Spectator {
                let score_left = name_xo + max_name_width_i as f32 + 1.0;
                let score_right = score_left + width_for_score_i as f32;
                if score_right - score_left > 5.0 {
                    if objective.render_type == RenderType::Hearts {
                        render_tablist_hearts(
                            renderer,
                            font,
                            score_left,
                            score_right,
                            yo,
                            info.uuid,
                            score_values[index],
                            health_states,
                            gui_tick,
                        );
                    } else if let Some((score_text, score_width)) = &score_texts[index] {
                        draw_formatted_chat_text_ex(
                            renderer,
                            font,
                            score_text,
                            score_right - score_width,
                            yo,
                            1.0,
                            1.0,
                            true,
                        );
                    }
                }
            }
        }
        draw_ping_icon(renderer, slot_width, xo, yo, info.latency);
    }
    if !footer_lines.is_empty() {
        yyo += rows as f32 * 9.0 + 1.0;
        renderer.draw_color_rect(
            bg_left,
            yyo - 1.0,
            bg_width,
            footer_lines.len() as f32 * 9.0 + 1.0,
            TAB_SECTION_BG,
        );
        for line in &footer_lines {
            let line_width_i = java_font_width(font, line);
            let text_x = (center_i - line_width_i / 2) as f32;
            draw_formatted_chat_text_ex(renderer, font, line, text_x, yyo, 1.0, 1.0, true);
            yyo += 9.0;
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub struct TabHealthState {
    last_value: i32,
    displayed_value: i32,
    last_update_tick: u32,
    blink_until_tick: u32,
}
impl TabHealthState {
    fn new(value: i32) -> Self {
        Self {
            last_value: value,
            displayed_value: value,
            last_update_tick: 0,
            blink_until_tick: 0,
        }
    }
    fn update(&mut self, value: i32, tick: u32) {
        if value != self.last_value {
            let duration = if value < self.last_value { 20 } else { 10 };
            self.blink_until_tick = tick.saturating_add(duration);
            self.last_value = value;
            self.last_update_tick = tick;
        }
        if tick.saturating_sub(self.last_update_tick) > 20 {
            self.displayed_value = value;
        }
    }
    fn is_blinking(self, tick: u32) -> bool {
        self.blink_until_tick > tick && self.blink_until_tick.saturating_sub(tick) % 6 >= 3
    }
}
fn render_tablist_hearts(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    left: f32,
    right: f32,
    y: f32,
    profile_id: Uuid,
    score: i32,
    health_states: &mut HashMap<Uuid, TabHealthState>,
    gui_tick: u32,
) {
    let health = health_states
        .entry(profile_id)
        .or_insert_with(|| TabHealthState::new(score));
    health.update(score, gui_tick);
    let displayed_value = health.displayed_value;
    let max_score = score.max(displayed_value);
    let full_hearts = (i64::from(max_score) + 1).div_euclid(2) as i32;
    let hearts_to_render = max_score.max(20) / 2;
    let blink = health.is_blinking(gui_tick);
    if full_hearts <= 0 {
        return;
    }
    let width_per_heart = (((right - left - 4.0) / hearts_to_render as f32).min(9.0)).floor();
    if width_per_heart <= 3.0 {
        let pct = (score as f32 / 20.0).clamp(0.0, 1.0);
        let hearts = score as f32 / 2.0;
        let hearts_text = format!("{hearts:.1}");
        let translated =
            crate::ui::i18n::translate("multiplayer.player.list.hp").replace("%s", &hearts_text);
        let text = if formatted_chat_width(font, &translated, 1.0) <= right - left {
            translated
        } else {
            hearts_text
        };
        let red = ((1.0 - pct) * 255.0) as u8;
        let green = (pct * 255.0) as u8;
        let text = format!("\u{1b}[38;2;{red};{green};0m{text}\u{1b}[m");
        let width = formatted_chat_width(font, &text, 1.0);
        let text_x = ((right + left - width) / 2.0).floor();
        draw_formatted_chat_text_ex(renderer, font, &text, text_x, y, 1.0, 1.0, false);
        return;
    }
    let container = ResourceLocation::from_string("minecraft:hud/heart/container");
    let container_blinking =
        ResourceLocation::from_string("minecraft:hud/heart/container_blinking");
    let full = ResourceLocation::from_string("minecraft:hud/heart/full");
    let half = ResourceLocation::from_string("minecraft:hud/heart/half");
    let full_blinking = ResourceLocation::from_string("minecraft:hud/heart/full_blinking");
    let half_blinking = ResourceLocation::from_string("minecraft:hud/heart/half_blinking");
    let absorbing_full_blinking =
        ResourceLocation::from_string("minecraft:hud/heart/absorbing_full_blinking");
    let absorbing_half_blinking =
        ResourceLocation::from_string("minecraft:hud/heart/absorbing_half_blinking");
    let container_sprite = if blink {
        &container_blinking
    } else {
        &container
    };
    let left_i = left.floor() as i32;
    let y_i = y.floor() as i32;
    let width_per_heart_i = width_per_heart as i32;
    for heart in full_hearts..hearts_to_render {
        let x = (left_i + heart * width_per_heart_i) as f32;
        renderer.draw_sprite(x, y_i as f32, 9.0, 9.0, container_sprite, WHITE);
    }
    for heart in 0..full_hearts {
        let x = (left_i + heart * width_per_heart_i) as f32;
        renderer.draw_sprite(x, y_i as f32, 9.0, 9.0, container_sprite, WHITE);
        if blink {
            if heart * 2 + 1 < displayed_value {
                renderer.draw_sprite(x, y_i as f32, 9.0, 9.0, &full_blinking, WHITE);
            } else if heart * 2 + 1 == displayed_value {
                renderer.draw_sprite(x, y_i as f32, 9.0, 9.0, &half_blinking, WHITE);
            }
        }
        if heart * 2 + 1 < score {
            renderer.draw_sprite(
                x,
                y_i as f32,
                9.0,
                9.0,
                if heart >= 10 {
                    &absorbing_full_blinking
                } else {
                    &full
                },
                WHITE,
            );
        } else if heart * 2 + 1 == score {
            renderer.draw_sprite(
                x,
                y_i as f32,
                9.0,
                9.0,
                if heart >= 10 {
                    &absorbing_half_blinking
                } else {
                    &half
                },
                WHITE,
            );
        }
    }
}
fn tablist_head_is_flipped(info: &PlayerInfoState, level_players: &HashSet<Uuid>) -> bool {
    level_players.contains(&info.uuid)
        && crate::render::entity::living_entity_renderer::LivingEntityRenderer::is_upside_down_name(
            &info.name,
        )
}
fn render_tablist_head(
    renderer: &mut GuiRenderer,
    info: &PlayerInfoState,
    x: f32,
    y: f32,
    level_players: &HashSet<Uuid>,
) {
    let _ = renderer.draw_player_face(
        x,
        y,
        8.0,
        info.uuid,
        info.texture_url.as_deref(),
        info.show_hat,
        tablist_head_is_flipped(info, level_players),
        [1.0, 1.0, 1.0, 1.0],
    );
}
fn draw_ping_icon(renderer: &mut GuiRenderer, slot_width: f32, xo: f32, yo: f32, latency: i32) {
    let sprite_name = if latency < 0 {
        "icon/ping_unknown"
    } else if latency < 150 {
        "icon/ping_5"
    } else if latency < 300 {
        "icon/ping_4"
    } else if latency < 600 {
        "icon/ping_3"
    } else if latency < 1000 {
        "icon/ping_2"
    } else {
        "icon/ping_1"
    };
    let sprite = ResourceLocation::from_string(&format!("minecraft:{sprite_name}"));
    let xo_i = xo.floor() as i32;
    let slot_width_i = slot_width.floor() as i32;
    let ping_x = (xo_i + slot_width_i - 11) as f32;
    let yo_i = yo.floor() as i32;
    renderer.draw_sprite(ping_x, yo_i as f32, 10.0, 8.0, &sprite, WHITE);
}
