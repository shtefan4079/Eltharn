use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::narration::{NarratedElementType, NarrationElementOutput};
use crate::framework::render_state::{RenderPipeline, TextureSetup};
use crate::framework::screen::{Screen, ScreenBase, screen_key_pressed, screen_key_released};
use crate::input::events::KeyEvent;
use crate::{FontWidthMeasurer, wrap_text};
use el_core::{Component, ResourceLocation};
use std::collections::BTreeSet;
use std::fmt;
pub const END_POEM_RESOURCE: &str = "minecraft:texts/end.txt";
pub const CREDITS_RESOURCE: &str = "minecraft:texts/credits.json";
pub const POSTCREDITS_RESOURCE: &str = "minecraft:texts/postcredits.txt";
const LINE_HEIGHT: i32 = 12;
const POEM_WRAP_WIDTH: f32 = 256.0;
const LOGO_X_OFFSET: i32 = 128;
const LOGO_Y_OFFSET: i32 = 150;
const END_SCREEN_EXTRA: f32 = 24.0;
const MENU_BACKGROUND_TEXTURE_SIZE: f32 = 32.0;
const MENU_BACKGROUND_RESOURCE: &str = "minecraft:gui/menu_background";
const VIGNETTE_RESOURCE: &str = "minecraft:misc/credits_vignette";
const OBFUSCATE_TOKEN: &str = "§f§k§a§b";
const OBFUSCATION_SEED: u64 = 8_124_371;
#[derive(Clone, Copy, Debug)]
pub struct WinScreenResources<'a> {
    pub credits_json: &'a str,
    pub end_poem: Option<&'a str>,
    pub postcredits: Option<&'a str>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinTextSpan {
    pub text: String,
    pub color: i32,
    pub obfuscated: bool,
    width: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinLine {
    pub spans: Vec<WinTextSpan>,
    pub centered: bool,
    pub narrated: bool,
    width: i32,
}
impl WinLine {
    pub fn text(&self) -> String {
        self.spans.iter().fold(String::new(), |mut text, span| {
            text.push_str(&span.text);
            text
        })
    }
    pub fn width(&self) -> i32 {
        self.width
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WinScreenAction {
    Finished,
}
#[derive(Debug)]
pub enum WinScreenLoadError {
    InvalidCreditsJson(serde_json::Error),
    Expected(&'static str),
    MissingResource(&'static str),
}
impl fmt::Display for WinScreenLoadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCreditsJson(error) => {
                write!(formatter, "could not parse {CREDITS_RESOURCE}: {error}")
            }
            Self::Expected(field) => {
                write!(formatter, "invalid {CREDITS_RESOURCE}: expected {field}")
            }
            Self::MissingResource(resource) => {
                write!(formatter, "missing required resource {resource}")
            }
        }
    }
}
impl std::error::Error for WinScreenLoadError {}
pub struct WinScreen {
    base: ScreenBase,
    poem: bool,
    lines: Vec<WinLine>,
    narrator_components: Vec<String>,
    total_scroll_length: i32,
    scroll: f32,
    unmodified_scroll_speed: f32,
    scroll_speed: f32,
    direction: f32,
    speedup_active: bool,
    speedup_modifiers: BTreeSet<i32>,
    finished: bool,
    pending_action: Option<WinScreenAction>,
}
impl WinScreen {
    pub fn from_resources(
        poem: bool,
        player_name: &str,
        resources: WinScreenResources<'_>,
        font: &dyn FontWidthMeasurer,
    ) -> Result<Self, WinScreenLoadError> {
        let mut lines = Vec::new();
        let mut narrator_components = Vec::new();
        if poem {
            let end_poem = resources
                .end_poem
                .ok_or(WinScreenLoadError::MissingResource(END_POEM_RESOURCE))?;
            append_poem_file(
                end_poem,
                player_name,
                font,
                &mut lines,
                &mut narrator_components,
            );
        }
        append_credits_file(
            resources.credits_json,
            font,
            &mut lines,
            &mut narrator_components,
        )?;
        if poem {
            let postcredits = resources
                .postcredits
                .ok_or(WinScreenLoadError::MissingResource(POSTCREDITS_RESOURCE))?;
            append_poem_file(
                postcredits,
                player_name,
                font,
                &mut lines,
                &mut narrator_components,
            );
        }
        let total_scroll_length = lines.len().saturating_mul(LINE_HEIGHT as usize) as i32;
        let unmodified_scroll_speed = if poem { 0.5 } else { 0.75 };
        let mut base = ScreenBase::new(Component::literal(""));
        base.set_pause_screen(!poem);
        Ok(Self {
            base,
            poem,
            lines,
            narrator_components,
            total_scroll_length,
            scroll: 0.0,
            unmodified_scroll_speed,
            scroll_speed: unmodified_scroll_speed,
            direction: 1.0,
            speedup_active: false,
            speedup_modifiers: BTreeSet::new(),
            finished: false,
            pending_action: None,
        })
    }
    pub fn poem(&self) -> bool {
        self.poem
    }
    pub fn lines(&self) -> &[WinLine] {
        &self.lines
    }
    pub fn total_scroll_length(&self) -> i32 {
        self.total_scroll_length
    }
    pub fn scroll(&self) -> f32 {
        self.scroll
    }
    pub fn scroll_speed(&self) -> f32 {
        self.scroll_speed
    }
    pub fn take_action(&mut self) -> Option<WinScreenAction> {
        self.pending_action.take()
    }
    fn recalculate_speed(&mut self) {
        let multiplier = if self.speedup_active {
            5.0 + self.speedup_modifiers.len() as f32 * 15.0
        } else {
            1.0
        };
        self.scroll_speed = self.unmodified_scroll_speed * multiplier * self.direction;
    }
    fn finish(&mut self) {
        if !self.finished {
            self.finished = true;
            self.pending_action = Some(WinScreenAction::Finished);
        }
    }
    fn end_scroll_threshold(&self) -> f32 {
        self.total_scroll_length as f32
            + self.base.height as f32
            + self.base.height as f32
            + END_SCREEN_EXTRA
    }
}
impl Screen for WinScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn tick(&mut self) {
        self.base.tick();
        if self.scroll > self.end_scroll_threshold() {
            self.finish();
        }
    }
    fn on_close(&mut self) {
        self.finish();
        self.base.request_close();
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        match event.key {
            KeyEvent::KEY_UP => self.direction = -1.0,
            341 | 345 => {
                self.speedup_modifiers.insert(event.key);
            }
            KeyEvent::KEY_SPACE => self.speedup_active = true,
            _ => {}
        }
        self.recalculate_speed();
        screen_key_pressed(self, event)
    }
    fn key_released(&mut self, event: KeyEvent) -> bool {
        match event.key {
            KeyEvent::KEY_UP => self.direction = 1.0,
            341 | 345 => {
                self.speedup_modifiers.remove(&event.key);
            }
            KeyEvent::KEY_SPACE => self.speedup_active = false,
            _ => {}
        }
        self.recalculate_speed();
        screen_key_released(self, event)
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        partial_tick: f32,
    ) {
        let width = self.base.width;
        let height = self.base.height;
        let background_scroll = self.scroll;
        if self.poem {
            graphics.resource_pipeline_blit(
                RenderPipeline::EndPortal,
                TextureSetup {
                    texture0: Some(ResourceLocation::from_string(
                        "minecraft:environment/end_sky",
                    )),
                    texture1: Some(ResourceLocation::from_string(
                        "minecraft:entity/end_portal/end_portal",
                    )),
                    texture2: None,
                },
                0,
                0,
                width,
                height,
            );
        } else {
            self.base.extract_background_backdrop(graphics, false);
            let v = background_scroll * 0.5;
            graphics.blit_resource_region(
                ResourceLocation::from_string(MENU_BACKGROUND_RESOURCE),
                0,
                0,
                width,
                height,
                0.0,
                v / MENU_BACKGROUND_TEXTURE_SIZE,
                width as f32 / MENU_BACKGROUND_TEXTURE_SIZE,
                (v + height as f32) / MENU_BACKGROUND_TEXTURE_SIZE,
                -1,
            );
        }
        graphics.resource_pipeline_blit(
            RenderPipeline::Vignette,
            TextureSetup::single_texture(ResourceLocation::from_string(VIGNETTE_RESOURCE)),
            0,
            0,
            width,
            height,
        );
        self.scroll = (self.scroll + partial_tick * self.scroll_speed).max(0.0);
        graphics.next_stratum();
        let first_y = height + LOGO_Y_OFFSET;
        let left_x = width / 2 - LOGO_X_OFFSET;
        let center_x = width / 2;
        let scroll = self.scroll;
        let last_index = self.lines.len().saturating_sub(1);
        graphics.pose_mut().push_pose();
        graphics.pose_mut().translate(0.0, -scroll);
        let logo_y = height + 50;
        graphics.blit_resource_region(
            ResourceLocation::from_string("minecraft:gui/title/minecraft"),
            left_x,
            logo_y,
            256,
            44,
            0.0,
            0.0,
            1.0,
            44.0 / 64.0,
            -1,
        );
        graphics.blit_resource_region(
            ResourceLocation::from_string("minecraft:gui/title/edition"),
            center_x - 64,
            logo_y + 37,
            128,
            14,
            0.0,
            0.0,
            1.0,
            14.0 / 16.0,
            -1,
        );
        for (index, line) in self.lines.iter().enumerate() {
            let y = first_y + index as i32 * LINE_HEIGHT;
            let screen_y = y as f32 - scroll;
            let final_line_y = if index == last_index {
                (height / 2 - 6) as f32
            } else {
                f32::NEG_INFINITY
            };
            let line_y = screen_y.max(final_line_y);
            if line_y + LINE_HEIGHT as f32 + 8.0 <= 0.0 || line_y >= height as f32 {
                continue;
            }
            let total_width = line.width;
            let mut x = if line.centered {
                center_x - total_width / 2
            } else {
                left_x
            };
            let y = y;
            let needs_final_adjustment = index == last_index && screen_y < final_line_y;
            if needs_final_adjustment {
                graphics.pose_mut().push_pose();
                graphics.pose_mut().translate(0.0, final_line_y - screen_y);
            }
            for span in &line.spans {
                let font = ResourceLocation::from_string("minecraft:default");
                if span.obfuscated {
                    graphics.text_obfuscated(font, span.text.clone(), x, y, span.color);
                } else {
                    graphics.text(font, span.text.clone(), x, y, span.color);
                }
                x += span.width;
            }
            if needs_final_adjustment {
                graphics.pose_mut().pop_pose();
            }
        }
        graphics.pose_mut().pop_pose();
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base.update_narration(output);
        for component in &self.narrator_components {
            output.add(NarratedElementType::Position, component.clone());
        }
    }
    fn is_pause_screen(&self) -> bool {
        !self.poem
    }
}
fn append_credits_file(
    json: &str,
    font: &dyn FontWidthMeasurer,
    lines: &mut Vec<WinLine>,
    narrator_components: &mut Vec<String>,
) -> Result<(), WinScreenLoadError> {
    let document: serde_json::Value =
        serde_json::from_str(json).map_err(WinScreenLoadError::InvalidCreditsJson)?;
    let sections = document
        .as_array()
        .ok_or(WinScreenLoadError::Expected("a top-level array"))?;
    for section in sections {
        let section_name = get_string(section, "section")?;
        append_credits_line(
            lines,
            narrator_components,
            "============",
            true,
            WHITE,
            false,
            font,
        );
        append_credits_line(
            lines,
            narrator_components,
            section_name,
            true,
            YELLOW,
            true,
            font,
        );
        append_credits_line(
            lines,
            narrator_components,
            "============",
            true,
            WHITE,
            false,
            font,
        );
        append_empty_line(lines, narrator_components, font);
        append_empty_line(lines, narrator_components, font);
        let disciplines = get_array(section, "disciplines")?;
        for discipline in disciplines {
            let discipline_name = get_string(discipline, "discipline")?;
            if !discipline_name.is_empty() {
                append_credits_line(
                    lines,
                    narrator_components,
                    discipline_name,
                    true,
                    YELLOW,
                    true,
                    font,
                );
                append_empty_line(lines, narrator_components, font);
                append_empty_line(lines, narrator_components, font);
            }
            for title in get_array(discipline, "titles")? {
                let title_name = get_string(title, "title")?;
                append_credits_line(
                    lines,
                    narrator_components,
                    title_name,
                    false,
                    GRAY,
                    true,
                    font,
                );
                for name in get_array(title, "names")? {
                    let name = name
                        .as_str()
                        .ok_or(WinScreenLoadError::Expected("a name string"))?;
                    let indented_name = format!("           {name}");
                    append_credits_line(
                        lines,
                        narrator_components,
                        &indented_name,
                        false,
                        WHITE,
                        true,
                        font,
                    );
                }
                append_empty_line(lines, narrator_components, font);
                append_empty_line(lines, narrator_components, font);
            }
        }
    }
    Ok(())
}
fn get_string<'a>(
    value: &'a serde_json::Value,
    field: &'static str,
) -> Result<&'a str, WinScreenLoadError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .ok_or(WinScreenLoadError::Expected("a string field"))
}
fn get_array<'a>(
    value: &'a serde_json::Value,
    field: &'static str,
) -> Result<&'a [serde_json::Value], WinScreenLoadError> {
    value
        .get(field)
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .ok_or(WinScreenLoadError::Expected("an array field"))
}
fn append_poem_file(
    contents: &str,
    player_name: &str,
    font: &dyn FontWidthMeasurer,
    lines: &mut Vec<WinLine>,
    narrator_components: &mut Vec<String>,
) {
    let mut obfuscation_random = LegacyJavaRandom::new(OBFUSCATION_SEED);
    for source_line in contents.lines() {
        let mut line = source_line.replace("PLAYERNAME", player_name);
        while let Some(index) = line.find(OBFUSCATE_TOKEN) {
            let count = 3 + obfuscation_random.next_int(4) as usize;
            line.replace_range(
                index..index + OBFUSCATE_TOKEN.len(),
                &format!("§f§k{}", "X".repeat(count)),
            );
        }
        append_wrapped_formatted_line(&line, font, lines);
        narrator_components.push(strip_formatting(&line));
        append_empty_line(lines, narrator_components, font);
    }
    for _ in 0..8 {
        append_empty_line(lines, narrator_components, font);
    }
}
struct LegacyJavaRandom {
    seed: u64,
}
impl LegacyJavaRandom {
    const MODULUS_MASK: u64 = (1 << 48) - 1;
    const MULTIPLIER: u64 = 25_214_903_917;
    const INCREMENT: u64 = 11;
    fn new(seed: u64) -> Self {
        Self {
            seed: (seed ^ Self::MULTIPLIER) & Self::MODULUS_MASK,
        }
    }
    fn next_bits(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT)
            & Self::MODULUS_MASK;
        (self.seed >> (48 - bits)) as u32
    }
    fn next_int(&mut self, bound: u32) -> u32 {
        debug_assert!(bound > 0);
        if bound.is_power_of_two() {
            return ((bound as u64 * self.next_bits(31) as u64) >> 31) as u32;
        }
        loop {
            let sample = self.next_bits(31) as i32;
            let remainder = sample % bound as i32;
            if sample
                .wrapping_sub(remainder)
                .wrapping_add(bound as i32 - 1)
                >= 0
            {
                return remainder as u32;
            }
        }
    }
}
fn append_wrapped_formatted_line(
    text: &str,
    font: &dyn FontWidthMeasurer,
    lines: &mut Vec<WinLine>,
) {
    let styled_chars = parse_legacy_formatting(text);
    let plain_text: String = styled_chars.iter().map(|styled| styled.character).collect();
    let wrapped = wrap_text(&plain_text, POEM_WRAP_WIDTH, font);
    let mut source_index = 0usize;
    for wrapped_line in wrapped {
        let mut spans = Vec::new();
        for expected in wrapped_line.chars() {
            while styled_chars
                .get(source_index)
                .is_some_and(|styled| styled.character == ' ')
                && styled_chars
                    .get(source_index)
                    .is_some_and(|styled| styled.character != expected)
            {
                source_index += 1;
            }
            let styled = styled_chars
                .get(source_index)
                .copied()
                .unwrap_or(StyledCharacter {
                    character: expected,
                    color: WHITE,
                    obfuscated: false,
                });
            source_index += 1;
            append_span_character(&mut spans, styled, font);
        }
        let visible_text = spans.iter().fold(String::new(), |mut text, span| {
            text.push_str(&span.text);
            text
        });
        lines.push(make_line(spans, false, false, font));
        debug_assert_eq!(visible_text, wrapped_line);
    }
}
#[derive(Clone, Copy)]
struct StyledCharacter {
    character: char,
    color: i32,
    obfuscated: bool,
}
fn parse_legacy_formatting(text: &str) -> Vec<StyledCharacter> {
    let mut output = Vec::new();
    let mut chars = text.chars();
    let mut color = WHITE;
    let mut obfuscated = false;
    while let Some(character) = chars.next() {
        if character == '§' {
            if let Some(code) = chars.next() {
                if let Some(next_color) = legacy_color(code) {
                    color = next_color;
                    obfuscated = false;
                } else if code.eq_ignore_ascii_case(&'k') {
                    obfuscated = true;
                } else if code.eq_ignore_ascii_case(&'r') {
                    color = WHITE;
                    obfuscated = false;
                }
            }
            continue;
        }
        output.push(StyledCharacter {
            character,
            color,
            obfuscated,
        });
    }
    output
}
fn strip_formatting(text: &str) -> String {
    parse_legacy_formatting(text)
        .iter()
        .map(|styled| styled.character)
        .collect()
}
fn legacy_color(code: char) -> Option<i32> {
    let rgb = match code.to_ascii_lowercase() {
        '0' => 0x000000,
        '1' => 0x0000AA,
        '2' => 0x00AA00,
        '3' => 0x00AAAA,
        '4' => 0xAA0000,
        '5' => 0xAA00AA,
        '6' => 0xFFAA00,
        '7' => 0xAAAAAA,
        '8' => 0x555555,
        '9' => 0x5555FF,
        'a' => 0x55FF55,
        'b' => 0x55FFFF,
        'c' => 0xFF5555,
        'd' => 0xFF55FF,
        'e' => 0xFFFF55,
        'f' => 0xFFFFFF,
        _ => return None,
    };
    Some((0xFF00_0000u32 | rgb) as i32)
}
fn append_credits_line(
    lines: &mut Vec<WinLine>,
    narrator_components: &mut Vec<String>,
    text: &str,
    centered: bool,
    color: i32,
    narrated: bool,
    font: &dyn FontWidthMeasurer,
) {
    let spans = if text.is_empty() {
        Vec::new()
    } else {
        vec![WinTextSpan {
            text: text.to_string(),
            color,
            obfuscated: false,
            width: font.measure_width(text, 1.0).ceil() as i32,
        }]
    };
    lines.push(make_line(spans, centered, narrated, font));
    if narrated {
        narrator_components.push(text.to_string());
    }
}
fn append_empty_line(
    lines: &mut Vec<WinLine>,
    narrator_components: &mut Vec<String>,
    font: &dyn FontWidthMeasurer,
) {
    lines.push(make_line(Vec::new(), false, false, font));
    narrator_components.push(String::new());
}
fn make_line(
    spans: Vec<WinTextSpan>,
    centered: bool,
    narrated: bool,
    font: &dyn FontWidthMeasurer,
) -> WinLine {
    let width = spans
        .iter()
        .map(|span| {
            if span.width == 0 {
                font.measure_width(&span.text, 1.0).ceil() as i32
            } else {
                span.width
            }
        })
        .sum();
    WinLine {
        spans,
        centered,
        narrated,
        width,
    }
}
fn append_span_character(
    spans: &mut Vec<WinTextSpan>,
    styled: StyledCharacter,
    font: &dyn FontWidthMeasurer,
) {
    if let Some(span) = spans
        .last_mut()
        .filter(|span| span.color == styled.color && span.obfuscated == styled.obfuscated)
    {
        span.text.push(styled.character);
        span.width = font.measure_width(&span.text, 1.0).ceil() as i32;
    } else {
        let text = styled.character.to_string();
        spans.push(WinTextSpan {
            width: font.measure_width(&text, 1.0).ceil() as i32,
            text,
            color: styled.color,
            obfuscated: styled.obfuscated,
        });
    }
}
const WHITE: i32 = 0xFFFF_FFFFu32 as i32;
const YELLOW: i32 = 0xFFFFFF55u32 as i32;
const GRAY: i32 = 0xFFAAAAAAu32 as i32;
