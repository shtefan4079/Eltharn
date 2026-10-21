use crate::ui::chat::signed_message::MessageSignature;
use crate::ui::domain::{ChatClickAction, ChatClickSpan, ChatHoverAction};
use crate::ui::gui::font::FontRenderer;
use crate::ui::gui::renderer::GuiRenderer;
use el_core::resource_location::ResourceLocation;
use std::collections::VecDeque;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatDisplayMode {
    Background,
    Foreground,
    ForegroundRestricted,
}
impl ChatDisplayMode {
    pub const fn focused(self) -> bool {
        !matches!(self, Self::Background)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatMethod {
    Message,
    Command,
}
impl ChatMethod {
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Message => "",
            Self::Command => "/",
        }
    }
    pub fn is_draft_restorable(self, draft: &Draft) -> bool {
        match self {
            Self::Message => true,
            Self::Command => draft.chat_method == Self::Command,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    pub text: String,
    pub chat_method: ChatMethod,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuiMessageSource {
    Player,
    SystemClient,
    SystemServer,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiMessageTagIcon {
    ChatModified,
}
impl GuiMessageTagIcon {
    pub const fn width(self) -> u32 {
        9
    }
    pub const fn height(self) -> u32 {
        9
    }
    pub fn sprite(self) -> &'static str {
        match self {
            GuiMessageTagIcon::ChatModified => "icon/chat_modified",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiMessageTag {
    pub indicator_color: u32,
    pub icon: Option<GuiMessageTagIcon>,
    pub text: Option<String>,
    pub log_tag: Option<String>,
}
impl GuiMessageTag {
    const COLOR_NOT_SECURE: u32 = 13_684_944;
    const COLOR_MODIFIED: u32 = 6_316_128;
    const COLOR_ERROR: u32 = 16_733_525;
    pub fn system() -> Self {
        Self {
            indicator_color: Self::COLOR_NOT_SECURE,
            icon: None,
            text: Some(crate::ui::i18n::translate("chat.tag.system").to_string()),
            log_tag: Some("System".to_string()),
        }
    }
    pub fn system_single_player() -> Self {
        Self {
            indicator_color: Self::COLOR_NOT_SECURE,
            icon: None,
            text: Some(crate::ui::i18n::translate("chat.tag.system_single_player").to_string()),
            log_tag: Some("System".to_string()),
        }
    }
    pub fn chat_not_secure() -> Self {
        Self {
            indicator_color: Self::COLOR_NOT_SECURE,
            icon: None,
            text: Some(crate::ui::i18n::translate("chat.tag.not_secure").to_string()),
            log_tag: Some("Not Secure".to_string()),
        }
    }
    pub fn chat_modified(original: impl Into<String>) -> Self {
        Self {
            indicator_color: Self::COLOR_MODIFIED,
            icon: Some(GuiMessageTagIcon::ChatModified),
            text: Some(format!(
                "{}\n{}",
                crate::ui::i18n::translate("chat.tag.modified"),
                original.into()
            )),
            log_tag: Some("Modified".to_string()),
        }
    }
    pub fn chat_error() -> Self {
        Self {
            indicator_color: Self::COLOR_ERROR,
            icon: None,
            text: Some(crate::ui::i18n::translate("chat.tag.error").to_string()),
            log_tag: Some("Chat Error".to_string()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuiMessage {
    pub added_time: u32,
    pub content: String,
    pub content_is_json_component: bool,
    pub click_spans: Vec<ChatClickSpan>,
    pub signature: Option<MessageSignature>,
    pub source: GuiMessageSource,
    pub tag: Option<GuiMessageTag>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatClickInteraction {
    pub action: Option<ChatClickAction>,
    pub hover: Option<ChatHoverAction>,
    pub insertion: Option<String>,
    pub source: GuiMessageSource,
}
#[derive(Debug, Clone, PartialEq)]
pub struct ChatRenderHover {
    pub tag_tooltip: Option<String>,
    pub queue_tooltip: Option<String>,
    pub restricted_tooltip: Option<String>,
    pub hover_action: Option<ChatHoverAction>,
    pub pointing_hand: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChatRenderOptions {
    pub opacity: f32,
    pub background_opacity: f32,
    pub scale: f32,
    pub line_spacing: f32,
    pub width: f32,
    pub height_focused: f32,
    pub height_unfocused: f32,
    pub colors: bool,
    pub show_player_messages: bool,
    pub show_server_messages: bool,
}
impl Default for ChatRenderOptions {
    fn default() -> Self {
        Self {
            opacity: 1.0,
            background_opacity: 0.5,
            scale: 1.0,
            line_spacing: 0.0,
            width: 1.0,
            height_focused: 1.0,
            height_unfocused: 0.4375,
            colors: true,
            show_player_messages: true,
            show_server_messages: true,
        }
    }
}
#[derive(Debug, Clone)]
struct DelayedMessageDeletion {
    signature: MessageSignature,
    deletable_after: u32,
}
#[derive(Debug, Clone)]
struct RenderedLine {
    runs: Vec<StyledRun>,
    added_time: u32,
    tag: Option<GuiMessageTag>,
    end_of_entry: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TextStyle {
    color: u32,
    bold: bool,
    italic: bool,
    underlined: bool,
    strikethrough: bool,
    obfuscated: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorldTextGlyph {
    pub ch: char,
    pub color: u32,
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
    pub obfuscated: bool,
}
impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: 0xFFFFFF,
            bold: false,
            italic: false,
            underlined: false,
            strikethrough: false,
            obfuscated: false,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
struct StyledRun {
    content: String,
    style: TextStyle,
    interaction: Option<ChatClickInteraction>,
}
#[derive(Debug, Clone)]
struct StyledChar {
    ch: char,
    style: TextStyle,
    interaction: Option<ChatClickInteraction>,
}
#[derive(Debug, Clone)]
struct StyledLine {
    content: String,
    runs: Vec<StyledRun>,
}
#[derive(Debug, Clone)]
pub struct ChatComponent {
    recent_chat: Vec<String>,
    messages: VecDeque<GuiMessage>,
    max_recent_chat: usize,
    max_messages: usize,
    chat_scrollbar_pos: usize,
    new_message_since_scroll: bool,
    latest_draft: Option<Draft>,
    deletion_queue: Vec<DelayedMessageDeletion>,
    delayed_messages: VecDeque<GuiMessage>,
    message_delay_ticks: u32,
    previous_message_tick: Option<u32>,
    cached_rendered_lines: Option<(ChatLayoutCacheKey, Vec<RenderedLine>)>,
    scroll_comp_pending: bool,
    scroll_comp_baseline: Option<usize>,
    scroll_comp_message_count: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChatLayoutCacheKey {
    max_width: u32,
    colors: bool,
    show_player_messages: bool,
    show_server_messages: bool,
}
impl Default for ChatComponent {
    fn default() -> Self {
        Self {
            recent_chat: Vec::new(),
            messages: VecDeque::new(),
            max_recent_chat: 100,
            max_messages: 100,
            chat_scrollbar_pos: 0,
            new_message_since_scroll: false,
            latest_draft: None,
            deletion_queue: Vec::new(),
            delayed_messages: VecDeque::new(),
            message_delay_ticks: 0,
            previous_message_tick: None,
            cached_rendered_lines: None,
            scroll_comp_pending: false,
            scroll_comp_baseline: None,
            scroll_comp_message_count: 0,
        }
    }
}
impl ChatComponent {
    pub const QUEUE_EXPAND_ID: &'static str = "internal/expand_chat_queue";
    pub const GO_TO_RESTRICTIONS_SCREEN: &'static str = "internal/go_to_restrictions_screen";
    pub const BOTTOM_MARGIN: f32 = 40.0;
    const MESSAGE_HEIGHT: f32 = 9.0;
    const MESSAGE_INDENT: f32 = 4.0;
    const MESSAGE_BOTTOM_TO_MESSAGE_TOP: f32 = 8.0;
    pub fn new() -> Self {
        Self::default()
    }
    pub fn tick(&mut self, current_tick: u32) {
        if self.message_delay_ticks == 0 {
            self.flush_delayed_messages(current_tick);
        } else if self.previous_message_tick.is_none_or(|previous| {
            current_tick.saturating_sub(previous) >= self.message_delay_ticks
        }) {
            self.accept_next_delayed_message(current_tick);
        }
        let mut ready = Vec::new();
        self.deletion_queue.retain(|entry| {
            if current_tick >= entry.deletable_after {
                ready.push(entry.signature.clone());
                false
            } else {
                true
            }
        });
        for signature in ready {
            self.replace_deleted_message(&signature);
        }
    }
    pub fn set_message_delay_ticks(&mut self, delay_ticks: u32, current_tick: u32) {
        let was_delayed = self.message_delay_ticks > 0;
        self.message_delay_ticks = delay_ticks;
        if was_delayed && delay_ticks == 0 {
            self.flush_delayed_messages(current_tick);
        }
    }
    pub fn queue_message(&mut self, message: GuiMessage, current_tick: u32) {
        let should_delay = self.message_delay_ticks > 0
            && self.previous_message_tick.is_some_and(|previous| {
                current_tick.saturating_sub(previous) < self.message_delay_ticks
            });
        if should_delay {
            self.delayed_messages.push_back(message);
        } else {
            self.add_message(message);
            self.previous_message_tick = Some(current_tick);
        }
    }
    pub fn queue_size(&self) -> usize {
        self.delayed_messages.len()
    }
    pub fn queue_hit_at(
        &self,
        font: &FontRenderer,
        screen_height: f32,
        options: ChatRenderOptions,
        mouse_x: f32,
        mouse_y: f32,
    ) -> bool {
        if self.delayed_messages.is_empty() {
            return false;
        }
        let scale = options.scale.clamp(0.01, 1.0);
        let chat_bottom = ((screen_height - Self::BOTTOM_MARGIN) / scale).floor() * scale;
        let queue_text = crate::ui::i18n::translate("chat.queue")
            .replace("%s", &self.delayed_messages.len().to_string());
        let text_left = Self::MESSAGE_INDENT as f32 * scale;
        let text_width = font.measure_width(&queue_text, scale);
        let text_top = chat_bottom + scale;
        let text_bottom = chat_bottom + 10.0 * scale;
        mouse_x >= text_left
            && mouse_x < text_left + text_width
            && mouse_y >= text_top
            && mouse_y < text_bottom
    }
    pub fn accept_next_delayed_message(&mut self, current_tick: u32) -> bool {
        let Some(message) = self.delayed_messages.pop_front() else {
            return false;
        };
        self.add_message(message);
        self.previous_message_tick = Some(current_tick);
        true
    }
    pub fn flush_delayed_messages(&mut self, current_tick: u32) {
        while let Some(message) = self.delayed_messages.pop_front() {
            self.add_message(message);
        }
        self.previous_message_tick = None;
        if !self.messages.is_empty() {
            self.previous_message_tick = Some(current_tick);
        }
    }
    pub fn add_recent_chat(&mut self, text: String) {
        if text.is_empty() {
            return;
        }
        if self.recent_chat.last() != Some(&text) {
            self.recent_chat.push(text);
            if self.recent_chat.len() > self.max_recent_chat {
                self.recent_chat.remove(0);
            }
        }
    }
    pub fn replace_recent_chat(&mut self, history: &[String]) {
        self.recent_chat.clear();
        self.recent_chat.extend(history.iter().cloned());
        while self.recent_chat.len() > self.max_recent_chat {
            self.recent_chat.remove(0);
        }
    }
    pub fn recent_chat(&self) -> &[String] {
        &self.recent_chat
    }
    pub fn add_message(&mut self, message: GuiMessage) {
        if self.chat_scrollbar_pos > 0 {
            self.new_message_since_scroll = true;
            if !self.scroll_comp_pending {
                self.scroll_comp_pending = true;
                self.scroll_comp_baseline = self
                    .cached_rendered_lines
                    .as_ref()
                    .map(|(_, lines)| lines.len());
                self.scroll_comp_message_count = 0;
            }
            self.scroll_comp_message_count += 1;
        }
        if let Some(tag) = message.tag.as_ref().and_then(|tag| tag.log_tag.as_ref()) {
            log::info!(
                "[{tag}] [CHAT] {}",
                plain_chat_text(&message.content)
                    .replace('\r', "\\r")
                    .replace('\n', "\\n")
            );
        } else {
            log::info!(
                "[CHAT] {}",
                plain_chat_text(&message.content)
                    .replace('\r', "\\r")
                    .replace('\n', "\\n")
            );
        }
        self.messages.push_back(message);
        while self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
        self.cached_rendered_lines = None;
    }
    pub fn add_client_system_message(&mut self, content: impl Into<String>, added_time: u32) {
        self.add_message(GuiMessage {
            added_time,
            content: content.into(),
            content_is_json_component: false,
            click_spans: Vec::new(),
            signature: None,
            source: GuiMessageSource::SystemClient,
            tag: Some(GuiMessageTag::system_single_player()),
        });
    }
    pub fn add_server_system_message(&mut self, content: impl Into<String>, added_time: u32) {
        self.add_server_system_message_with_click_spans(content, Vec::new(), added_time);
    }
    pub fn add_server_system_message_with_click_spans(
        &mut self,
        content: impl Into<String>,
        click_spans: Vec<ChatClickSpan>,
        added_time: u32,
    ) {
        self.add_message(GuiMessage {
            added_time,
            content: content.into(),
            content_is_json_component: false,
            click_spans,
            signature: None,
            source: GuiMessageSource::SystemServer,
            tag: Some(GuiMessageTag::system_single_player()),
        });
    }
    pub fn add_player_message(
        &mut self,
        content: impl Into<String>,
        signature: Option<MessageSignature>,
        tag: Option<GuiMessageTag>,
        added_time: u32,
    ) {
        self.add_message(GuiMessage {
            added_time,
            content: content.into(),
            content_is_json_component: false,
            click_spans: Vec::new(),
            signature,
            source: GuiMessageSource::Player,
            tag,
        });
    }
    pub fn messages(&self) -> impl Iterator<Item = &GuiMessage> {
        self.messages.iter()
    }
    pub fn clear_messages(&mut self, clear_history: bool, command_history: &[String]) {
        self.messages.clear();
        self.delayed_messages.clear();
        self.deletion_queue.clear();
        self.cached_rendered_lines = None;
        self.previous_message_tick = None;
        self.reset_chat_scroll();
        if clear_history {
            self.recent_chat.clear();
            self.recent_chat.extend_from_slice(command_history);
        }
    }
    pub fn rescale_chat(&mut self) {
        self.reset_chat_scroll();
        self.refresh_trimmed_messages();
    }
    pub fn refresh_trimmed_messages(&mut self) {
        self.cached_rendered_lines = None;
    }
    pub fn delete_message(&mut self, signature: &MessageSignature, current_tick: u32) {
        if let Some(index) = self
            .delayed_messages
            .iter()
            .position(|message| message.signature.as_ref() == Some(signature))
        {
            self.delayed_messages.remove(index);
            return;
        }
        let Some(message) = self
            .messages
            .iter()
            .find(|message| message.signature.as_ref() == Some(signature))
        else {
            return;
        };
        let deletable_after = message.added_time.saturating_add(60);
        if current_tick >= deletable_after {
            self.replace_deleted_message(signature);
        } else if !self
            .deletion_queue
            .iter()
            .any(|entry| &entry.signature == signature)
        {
            self.deletion_queue.push(DelayedMessageDeletion {
                signature: signature.clone(),
                deletable_after,
            });
        }
    }
    fn replace_deleted_message(&mut self, signature: &MessageSignature) {
        if let Some(message) = self
            .messages
            .iter_mut()
            .find(|message| message.signature.as_ref() == Some(signature))
        {
            message.content = format!(
                "\u{1b}[38;2;170;170;170m\u{1b}[3m{}\u{1b}[m",
                crate::ui::i18n::translate("chat.deleted_marker")
            );
            message.content_is_json_component = false;
            message.signature = None;
            message.source = GuiMessageSource::SystemServer;
            message.tag = Some(GuiMessageTag::system());
            message.click_spans.clear();
            self.cached_rendered_lines = None;
        }
    }
    pub fn save_as_draft(&mut self, text: String) {
        if text.trim().is_empty() {
            self.latest_draft = None;
            return;
        }
        let chat_method = if text.starts_with('/') {
            ChatMethod::Command
        } else {
            ChatMethod::Message
        };
        self.latest_draft = Some(Draft { text, chat_method });
    }
    pub fn take_draft_for_method(&mut self, chat_method: ChatMethod) -> Option<String> {
        if let Some(draft) = &self.latest_draft {
            if chat_method.is_draft_restorable(draft) {
                return self.latest_draft.take().map(|d| d.text);
            }
        }
        None
    }
    pub fn draft_text_for_method(&self, chat_method: ChatMethod) -> Option<String> {
        self.latest_draft
            .as_ref()
            .filter(|draft| chat_method.is_draft_restorable(draft))
            .map(|draft| draft.text.clone())
    }
    pub fn take_draft(&mut self) -> Option<String> {
        self.take_draft_for_method(ChatMethod::Message)
    }
    pub fn discard_draft(&mut self) {
        self.latest_draft = None;
    }
    pub fn reset_chat_scroll(&mut self) {
        self.chat_scrollbar_pos = 0;
        self.new_message_since_scroll = false;
        self.scroll_comp_pending = false;
        self.scroll_comp_baseline = None;
        self.scroll_comp_message_count = 0;
    }
    pub fn scroll_chat(
        &mut self,
        amount: i32,
        font: &FontRenderer,
        options: ChatRenderOptions,
        focused: bool,
    ) {
        let total = self.rendered_lines(font, options, focused).len();
        let page = self.lines_per_page(options, focused);
        let max_scroll = total.saturating_sub(page);
        self.chat_scrollbar_pos =
            (self.chat_scrollbar_pos as i32 + amount).clamp(0, max_scroll as i32) as usize;
        if self.chat_scrollbar_pos == 0 {
            self.new_message_since_scroll = false;
        }
    }
    pub fn lines_per_page(&self, options: ChatRenderOptions, focused: bool) -> usize {
        let height = chat_height(if focused {
            options.height_focused
        } else {
            options.height_unfocused
        });
        (height / line_height(options.line_spacing)).max(1)
    }
    pub fn click_interaction_at(
        &mut self,
        font: &FontRenderer,
        screen_height: f32,
        options: ChatRenderOptions,
        mouse_x: f32,
        mouse_y: f32,
    ) -> Option<ChatClickInteraction> {
        let scale = options.scale.clamp(0.01, 1.0);
        let entry_height = line_height(options.line_spacing) as f32 * scale;
        let chat_bottom_unscaled = ((screen_height - Self::BOTTOM_MARGIN) / scale).floor();
        let chat_bottom = chat_bottom_unscaled * scale;
        let pose_x = Self::MESSAGE_INDENT * scale;
        let scrollbar_pos = self.chat_scrollbar_pos;
        let per_page = self.lines_per_page(options, true);
        let lines = self.rendered_lines(font, options, true);
        let visible_start = scrollbar_pos.min(lines.len());
        let visible_end = (visible_start + per_page).min(lines.len());
        for (line_index, line) in lines[visible_start..visible_end].iter().enumerate() {
            let bottom = chat_bottom - line_index as f32 * entry_height;
            let text_top = bottom - message_top_offset(options.line_spacing) * scale;
            if mouse_y < text_top || mouse_y >= text_top + 9.0 * scale {
                continue;
            }
            let mut cursor_x = pose_x;
            for run in &line.runs {
                let width = styled_run_width(font, &run.content, run.style.bold, scale);
                if mouse_x >= cursor_x && mouse_x < cursor_x + width {
                    return run.interaction.clone();
                }
                cursor_x += width;
            }
            return None;
        }
        None
    }
    pub fn restricted_prompt_at(
        &mut self,
        font: &FontRenderer,
        screen_height: f32,
        options: ChatRenderOptions,
        mouse_x: f32,
        mouse_y: f32,
    ) -> bool {
        let scale = options.scale.clamp(0.01, 1.0);
        let entry_height = line_height(options.line_spacing) as f32 * scale;
        let chat_bottom_unscaled = ((screen_height - Self::BOTTOM_MARGIN) / scale).floor();
        let chat_bottom = chat_bottom_unscaled * scale;
        let per_page = self.lines_per_page(options, true);
        let scrollbar_pos = self.chat_scrollbar_pos;
        let lines = self.rendered_lines(font, options, true);
        let visible_start = scrollbar_pos.min(lines.len());
        let visible_end = (visible_start + per_page).min(lines.len());
        let visible_alpha_count = visible_end - visible_start;
        let line_above = chat_bottom - (visible_alpha_count + 1) as f32 * entry_height;
        let message_y_offset = message_top_offset(options.line_spacing) * scale;
        let text_top = line_above + entry_height - message_y_offset - scale;
        if mouse_y < text_top || mouse_y >= text_top + 9.0 * scale {
            return false;
        }
        let full_text = restricted_message_text();
        let max_width_unscaled = (chat_width(options.width) as f32 / scale).ceil();
        let text = if formatted_chat_width(font, &full_text, 1.0).ceil() > max_width_unscaled {
            clip_formatted_chat_text(font, &full_text, max_width_unscaled as f32)
        } else {
            full_text
        };
        let pose_x = Self::MESSAGE_INDENT * scale;
        let text_width = formatted_chat_width(font, &text, scale);
        mouse_x >= pose_x && mouse_x < pose_x + text_width
    }
    pub fn render(
        &mut self,
        renderer: &mut GuiRenderer,
        font: &FontRenderer,
        screen_height: f32,
        current_tick: u32,
        mode: ChatDisplayMode,
        options: ChatRenderOptions,
        mouse_x: f32,
        mouse_y: f32,
        insertion_cursor: bool,
    ) -> ChatRenderHover {
        let scale = options.scale.clamp(0.01, 1.0);
        let focused = mode.focused();
        let is_restricted = mode == ChatDisplayMode::ForegroundRestricted;
        let scrollbar_pos = self.chat_scrollbar_pos;
        let new_message_since_scroll = self.new_message_since_scroll;
        let delayed_message_count = self.delayed_messages.len();
        let per_page = self.lines_per_page(options, focused);
        let lines = self.rendered_lines(font, options, focused);
        if lines.is_empty() && delayed_message_count == 0 && !is_restricted {
            return ChatRenderHover {
                tag_tooltip: None,
                queue_tooltip: None,
                restricted_tooltip: None,
                hover_action: None,
                pointing_hand: false,
            };
        }
        let text_opacity = options.opacity.clamp(0.0, 1.0) * 0.9 + 0.1;
        let background_opacity = options.background_opacity.clamp(0.0, 1.0);
        let entry_height_unscaled = line_height(options.line_spacing) as f32;
        let entry_height = entry_height_unscaled * scale;
        let message_y_offset_unscaled = message_top_offset(options.line_spacing);
        let message_y_offset = message_y_offset_unscaled * scale;
        let chat_width_unscaled = chat_width(options.width) as f32;
        let max_width_unscaled = (chat_width_unscaled / scale).ceil();
        let chat_bottom_unscaled = ((screen_height - Self::BOTTOM_MARGIN) / scale).floor();
        let chat_bottom = chat_bottom_unscaled * scale;
        let pose_x = Self::MESSAGE_INDENT * scale;
        let visible_start = scrollbar_pos.min(lines.len());
        let visible_end = (visible_start + per_page).min(lines.len());
        let visible = &lines[visible_start..visible_end];
        let visible_alpha_count = visible
            .iter()
            .filter(|line| {
                let alpha = if focused {
                    1.0
                } else {
                    message_alpha(current_tick, line.added_time)
                };
                alpha > 0.00001
            })
            .count();
        let mut hovered_tag_tooltip: Option<String> = None;
        let mut hovered_queue_tooltip: Option<String> = None;
        let mut hovered_restricted_tooltip: Option<String> = None;
        let mut hovered_action: Option<ChatHoverAction> = None;
        let mut hovered_this_message = false;
        let mut pointing_hand = false;
        for (line_index, line) in visible.iter().enumerate() {
            let alpha = if focused {
                1.0
            } else {
                message_alpha(current_tick, line.added_time)
            };
            if alpha <= 0.00001 {
                continue;
            }
            let bottom = chat_bottom - line_index as f32 * entry_height;
            let top = bottom - entry_height;
            renderer.draw_rect(
                0.0,
                top,
                (max_width_unscaled + 12.0) * scale,
                entry_height,
                [0.0, 0.0, 0.0, java_alpha(alpha * background_opacity)],
            );
        }
        if delayed_message_count > 0 {
            renderer.draw_rect(
                2.0 * scale,
                chat_bottom,
                (max_width_unscaled + 6.0) * scale,
                Self::MESSAGE_HEIGHT * scale,
                [0.0, 0.0, 0.0, java_alpha(background_opacity)],
            );
        }
        if is_restricted {
            let line_above = chat_bottom - (visible_alpha_count + 1) as f32 * entry_height;
            renderer.draw_rect(
                2.0 * scale,
                line_above,
                (max_width_unscaled + 10.0) * scale,
                entry_height,
                [0.0, 0.0, 0.0, java_alpha(background_opacity)],
            );
        }
        for (line_index, line) in visible.iter().enumerate() {
            let alpha = if focused {
                1.0
            } else {
                message_alpha(current_tick, line.added_time)
            };
            if alpha <= 0.00001 {
                continue;
            }
            let bottom = chat_bottom - line_index as f32 * entry_height;
            let top = bottom - entry_height;
            let text_top = bottom - message_y_offset;
            if focused {
                let text_width = styled_runs_width(font, &line.runs, scale);
                let hovered_text = mouse_x >= pose_x
                    && mouse_x < pose_x + text_width
                    && mouse_y >= text_top
                    && mouse_y < text_top + 9.0 * scale;
                hovered_this_message |= hovered_text;
                if hovered_text {
                    if let Some(hovered_interaction) =
                        hovered_run_interaction(font, &line.runs, pose_x, mouse_x, scale)
                    {
                        if let Some(hover) = &hovered_interaction.hover {
                            hovered_action = Some(hover.clone());
                        }
                        if hovered_interaction.action.is_some() {
                            pointing_hand = true;
                        }
                    }
                }
                if insertion_cursor
                    && hovered_text
                    && run_with_insertion_hovered(font, &line.runs, pose_x, mouse_x, scale)
                {
                    pointing_hand = true;
                }
                if let Some(tag) = &line.tag {
                    let strip_width = (2.0 * scale).max(1.0);
                    if mouse_x >= 0.0 && mouse_x < strip_width && mouse_y >= top && mouse_y < bottom
                    {
                        hovered_tag_tooltip = tag_tooltip_text(font, tag);
                    }
                }
            }
            if let Some(tag) = &line.tag {
                let [r, g, b] = rgb(tag.indicator_color);
                renderer.draw_rect(
                    0.0,
                    top,
                    (2.0 * scale).max(1.0),
                    entry_height,
                    [r, g, b, java_alpha(alpha * text_opacity)],
                );
            }
            draw_styled_runs(
                renderer,
                font,
                &line.runs,
                pose_x,
                text_top,
                java_alpha(alpha * text_opacity),
                scale,
                false,
                true,
                None,
            );
            if focused && line.end_of_entry {
                let force_icon = std::mem::replace(&mut hovered_this_message, false);
                if let Some((tag, icon)) = line
                    .tag
                    .as_ref()
                    .and_then(|tag| tag.icon.as_ref().map(|icon| (tag, icon)))
                {
                    let icon_left = pose_x
                        + styled_runs_width(font, &line.runs, 1.0).ceil() * scale
                        + Self::MESSAGE_INDENT * scale;
                    let icon_width = icon.width() as f32 * scale;
                    let icon_height = icon.height() as f32 * scale;
                    let icon_top = text_top - scale;
                    let icon_bottom = text_top + 9.0 * scale;
                    let icon_right = icon_left + icon_width;
                    let icon_hovered = mouse_x >= icon_left
                        && mouse_x < icon_right
                        && mouse_y >= icon_top
                        && mouse_y < icon_bottom;
                    if icon_hovered {
                        hovered_tag_tooltip = tag_tooltip_text(font, tag);
                    }
                    if force_icon || icon_hovered {
                        let sprite = ResourceLocation::new("minecraft", icon.sprite());
                        renderer.draw_sprite(
                            icon_left,
                            icon_top,
                            icon_width,
                            icon_height,
                            &sprite,
                            [1.0, 1.0, 1.0, 1.0],
                        );
                    }
                }
            }
        }
        if delayed_message_count > 0 {
            let queue_text = crate::ui::i18n::translate("chat.queue")
                .replace("%s", &delayed_message_count.to_string());
            renderer.draw_text_top_with_shadow(
                font,
                &queue_text,
                pose_x,
                chat_bottom + scale,
                [1.0, 1.0, 1.0, java_alpha(0.5 * text_opacity)],
                scale,
            );
            if focused {
                let queue_width = font.measure_width(&queue_text, scale);
                let queue_text_top = chat_bottom + scale;
                let queue_text_bottom = chat_bottom + 10.0 * scale;
                if mouse_x >= pose_x
                    && mouse_x < pose_x + queue_width
                    && mouse_y >= queue_text_top
                    && mouse_y < queue_text_bottom
                {
                    hovered_queue_tooltip =
                        Some(crate::ui::i18n::translate("chat.queue.tooltip").to_string());
                }
            }
        }
        if is_restricted {
            let line_above = chat_bottom - (visible_alpha_count + 1) as f32 * entry_height;
            let full_text = restricted_message_text();
            let full_width = formatted_chat_width(font, &full_text, 1.0).ceil();
            let text = if full_width > max_width_unscaled {
                clip_formatted_chat_text(font, &full_text, max_width_unscaled as f32)
            } else {
                full_text
            };
            let runs = styled_line_from_chars(&parse_ansi_chars(&text)).runs;
            if focused
                && full_width > max_width_unscaled
                && mouse_x >= pose_x
                && mouse_x < pose_x + formatted_chat_width(font, &text, scale)
                && mouse_y >= line_above
                && mouse_y < line_above + entry_height
            {
                hovered_restricted_tooltip =
                    Some(crate::ui::i18n::translate("chat_screen.restricted").to_string());
            }
            draw_styled_runs(
                renderer,
                font,
                &runs,
                pose_x,
                line_above + entry_height - message_y_offset - scale,
                java_alpha(text_opacity),
                scale,
                false,
                true,
                None,
            );
        }
        if focused && lines.len() > per_page {
            let entry_height_unscaled = line_height(options.line_spacing);
            let chat_height_unscaled = visible_alpha_count * entry_height_unscaled;
            let virtual_height_unscaled = lines.len() * entry_height_unscaled;
            if chat_height_unscaled > 0 && virtual_height_unscaled != chat_height_unscaled {
                let scroll_offset_unscaled = scrollbar_pos * chat_height_unscaled / lines.len();
                let bar_height_unscaled =
                    chat_height_unscaled * chat_height_unscaled / virtual_height_unscaled;
                let scroll_y_unscaled = scroll_offset_unscaled as i32 - chat_bottom_unscaled as i32;
                let alpha = if scroll_y_unscaled > 0 {
                    170.0 / 255.0
                } else {
                    96.0 / 255.0
                };
                let main_rgb = if new_message_since_scroll {
                    rgb(13_382_451)
                } else {
                    rgb(3_355_562)
                };
                let highlight_rgb = rgb(13_421_772);
                let scroll_x = (max_width_unscaled + 8.0) * scale;
                let scroll_top =
                    (-scroll_y_unscaled) as f32 * scale - bar_height_unscaled as f32 * scale;
                let bar_height = bar_height_unscaled as f32 * scale;
                renderer.draw_rect(
                    scroll_x,
                    scroll_top,
                    (2.0 * scale).max(1.0),
                    bar_height,
                    [main_rgb[0], main_rgb[1], main_rgb[2], alpha],
                );
                renderer.draw_rect(
                    scroll_x + (1.0 * scale).max(1.0),
                    scroll_top,
                    (1.0 * scale).max(1.0),
                    bar_height,
                    [highlight_rgb[0], highlight_rgb[1], highlight_rgb[2], alpha],
                );
            }
        }
        ChatRenderHover {
            tag_tooltip: hovered_tag_tooltip,
            queue_tooltip: hovered_queue_tooltip,
            restricted_tooltip: hovered_restricted_tooltip,
            hover_action: hovered_action,
            pointing_hand,
        }
    }
    fn rendered_lines(
        &mut self,
        font: &FontRenderer,
        options: ChatRenderOptions,
        focused: bool,
    ) -> &[RenderedLine] {
        let scale = options.scale.clamp(0.01, 1.0);
        let max_width = (chat_width(options.width) as f32 / scale).floor().max(1.0);
        let cache_key = ChatLayoutCacheKey {
            max_width: max_width as u32,
            colors: options.colors,
            show_player_messages: options.show_player_messages,
            show_server_messages: options.show_server_messages,
        };
        if !self.scroll_comp_pending
            && self
                .cached_rendered_lines
                .as_ref()
                .is_some_and(|(key, _)| *key == cache_key)
        {
            return &self
                .cached_rendered_lines
                .as_ref()
                .expect("checked above")
                .1;
        }
        let mut output = Vec::new();
        let mut new_messages_lines = 0usize;
        for (msg_idx, message) in self.messages.iter().rev().enumerate() {
            if matches!(message.source, GuiMessageSource::Player) && !options.show_player_messages {
                continue;
            }
            if matches!(message.source, GuiMessageSource::SystemServer)
                && !options.show_server_messages
            {
                continue;
            }
            let icon_width = if message.tag.as_ref().and_then(|tag| tag.icon).is_some() {
                15.0
            } else {
                0.0
            };
            let content = if options.colors {
                message.content.clone()
            } else {
                plain_chat_text(&message.content)
            };
            let wrapped = wrap_text_with_click_spans(
                font,
                &content,
                &message.click_spans,
                (max_width - icon_width).max(1.0),
                message.source,
                message.content_is_json_component,
            );
            let last_index = wrapped.len().saturating_sub(1);
            if self.scroll_comp_pending && msg_idx < self.scroll_comp_message_count {
                new_messages_lines += wrapped.len();
            }
            for (index, line) in wrapped.into_iter().enumerate().rev() {
                output.push(RenderedLine {
                    runs: line.runs,
                    added_time: message.added_time,
                    tag: message.tag.clone(),
                    end_of_entry: index == last_index,
                });
            }
        }
        output.truncate(100);
        if self.scroll_comp_pending {
            let delta = if self.scroll_comp_baseline.is_some() {
                new_messages_lines
            } else {
                self.scroll_comp_message_count
            };
            let max_scroll = output
                .len()
                .saturating_sub(self.lines_per_page(options, focused));
            self.chat_scrollbar_pos = (self.chat_scrollbar_pos + delta).min(max_scroll);
            if self.chat_scrollbar_pos == 0 {
                self.new_message_since_scroll = false;
            }
            self.scroll_comp_pending = false;
            self.scroll_comp_baseline = None;
            self.scroll_comp_message_count = 0;
        }
        self.cached_rendered_lines = Some((cache_key, output));
        &self
            .cached_rendered_lines
            .as_ref()
            .expect("just populated")
            .1
    }
}
fn chat_width(value: f32) -> i32 {
    (value.clamp(0.0, 1.0) * 280.0 + 40.0).floor() as i32
}
fn chat_height(value: f32) -> usize {
    (value.clamp(0.0, 1.0) * 160.0 + 20.0).floor() as usize
}
fn line_height(spacing: f32) -> usize {
    (9.0 * (spacing.clamp(0.0, 1.0) + 1.0)).floor() as usize
}
fn message_top_offset(spacing: f32) -> f32 {
    (ChatComponent::MESSAGE_BOTTOM_TO_MESSAGE_TOP * (spacing.clamp(0.0, 1.0) + 1.0)
        - ChatComponent::MESSAGE_INDENT * spacing.clamp(0.0, 1.0))
    .round()
}
fn message_alpha(current_tick: u32, added_time: u32) -> f32 {
    let age = current_tick.saturating_sub(added_time) as f64;
    let mut value = (1.0 - age / 200.0) * 10.0;
    value = value.clamp(0.0, 1.0);
    (value * value) as f32
}
fn restricted_message_text() -> String {
    format!(
        "\u{1b}[38;2;255;85;85m\u{1b}[4m{}\u{1b}[m",
        crate::ui::i18n::translate("chat_screen.restricted")
    )
}
fn clip_formatted_chat_text(font: &FontRenderer, text: &str, width: f32) -> String {
    let ellipsis = "...";
    let ellipsis_width = styled_run_width(font, ellipsis, false, 1.0);
    let mut chars = parse_ansi_chars(text);
    let mut remaining = (width - ellipsis_width).max(0.0);
    let keep = chars
        .iter()
        .position(|styled| {
            remaining -= styled_char_width(font, styled);
            remaining < 0.0
        })
        .unwrap_or(chars.len());
    chars.truncate(keep);
    let mut out = styled_line_to_ansi(&styled_line_from_chars(&chars));
    out.push_str(ellipsis);
    out
}
fn wrap_text(font: &FontRenderer, text: &str, max_width: f32) -> Vec<StyledLine> {
    wrap_text_inner(
        font,
        text,
        &[],
        max_width,
        GuiMessageSource::SystemClient,
        true,
        false,
    )
}
fn wrap_text_with_click_spans(
    font: &FontRenderer,
    text: &str,
    click_spans: &[ChatClickSpan],
    max_width: f32,
    source: GuiMessageSource,
    content_is_json_component: bool,
) -> Vec<StyledLine> {
    wrap_text_inner(
        font,
        text,
        click_spans,
        max_width,
        source,
        true,
        content_is_json_component,
    )
}
fn wrap_text_inner(
    font: &FontRenderer,
    text: &str,
    click_spans: &[ChatClickSpan],
    max_width: f32,
    source: GuiMessageSource,
    indent_wrapped: bool,
    content_is_json_component: bool,
) -> Vec<StyledLine> {
    let styled =
        parse_ansi_chars_with_click_spans(text, click_spans, source, content_is_json_component);
    let mut output = Vec::new();
    let mut paragraph = Vec::new();
    for styled_char in styled {
        if styled_char.ch == '\n' {
            wrap_paragraph(font, &paragraph, max_width, &mut output, indent_wrapped);
            paragraph.clear();
        } else {
            paragraph.push(styled_char);
        }
    }
    wrap_paragraph(font, &paragraph, max_width, &mut output, indent_wrapped);
    if output.is_empty() {
        output.push(styled_line_from_chars(&[]));
    }
    output
}
fn wrap_paragraph(
    font: &FontRenderer,
    paragraph: &[StyledChar],
    max_width: f32,
    output: &mut Vec<StyledLine>,
    indent_wrapped: bool,
) {
    if paragraph.is_empty() {
        output.push(styled_line_from_chars(&[]));
        return;
    }
    let mut line = Vec::new();
    let mut line_width = 0.0;
    let mut word = Vec::new();
    let start_len = output.len();
    let flush_line = |line: &mut Vec<StyledChar>, output: &mut Vec<StyledLine>| {
        if !line.is_empty() {
            if indent_wrapped && output.len() > start_len {
                line.insert(
                    0,
                    StyledChar {
                        ch: ' ',
                        style: TextStyle::default(),
                        interaction: None,
                    },
                );
            }
            output.push(styled_line_from_chars(line));
        } else if output.len() == start_len {
            output.push(styled_line_from_chars(line));
        }
        line.clear();
    };
    let append_word = |word: &mut Vec<StyledChar>,
                       line: &mut Vec<StyledChar>,
                       line_width: &mut f32,
                       output: &mut Vec<StyledLine>| {
        if word.is_empty() {
            return;
        }
        let word_width = styled_chars_width(font, word);
        if !line.is_empty() && *line_width + word_width > max_width {
            flush_line(line, output);
            *line_width = 0.0;
        }
        for styled in word.drain(..) {
            let char_width = styled_char_width(font, &styled);
            if !line.is_empty() && *line_width + char_width > max_width {
                flush_line(line, output);
                *line_width = 0.0;
            }
            line.push(styled);
            *line_width += char_width;
        }
    };
    for styled in paragraph.iter().cloned() {
        let is_space = styled.ch == ' ';
        word.push(styled);
        if is_space {
            append_word(&mut word, &mut line, &mut line_width, output);
        }
    }
    append_word(&mut word, &mut line, &mut line_width, output);
    if !line.is_empty() {
        flush_line(&mut line, output);
    }
}
fn styled_chars_width(font: &FontRenderer, chars: &[StyledChar]) -> f32 {
    chars
        .iter()
        .map(|styled| styled_char_width(font, styled))
        .sum()
}
fn styled_char_width(font: &FontRenderer, styled: &StyledChar) -> f32 {
    font.glyph(styled.ch).advance + if styled.style.bold { 1.0 } else { 0.0 }
}
fn styled_line_from_chars(chars: &[StyledChar]) -> StyledLine {
    let mut content = String::new();
    let mut runs: Vec<StyledRun> = Vec::new();
    for styled in chars {
        content.push(styled.ch);
        if let Some(run) = runs
            .last_mut()
            .filter(|run| run.style == styled.style && run.interaction == styled.interaction)
        {
            run.content.push(styled.ch);
        } else {
            runs.push(StyledRun {
                content: styled.ch.to_string(),
                style: styled.style,
                interaction: styled.interaction.clone(),
            });
        }
    }
    StyledLine { content, runs }
}
fn parse_ansi_chars(text: &str) -> Vec<StyledChar> {
    parse_ansi_chars_with_click_spans(text, &[], GuiMessageSource::SystemClient, true)
}
pub(crate) fn ansi_world_glyphs(text: &str) -> Vec<WorldTextGlyph> {
    parse_ansi_chars(text)
        .into_iter()
        .map(|styled| WorldTextGlyph {
            ch: styled.ch,
            color: styled.style.color,
            bold: styled.style.bold,
            italic: styled.style.italic,
            underlined: styled.style.underlined,
            strikethrough: styled.style.strikethrough,
            obfuscated: styled.style.obfuscated,
        })
        .collect()
}
fn ansi_256_to_rgb(code: u32) -> u32 {
    match code {
        0 => 0x000000,
        1 => 0xAA0000,
        2 => 0x00AA00,
        3 => 0xFFAA00,
        4 => 0x0000AA,
        5 => 0xAA00AA,
        6 => 0x00AAAA,
        7 => 0xAAAAAA,
        8 => 0x555555,
        9 => 0xFF5555,
        10 => 0x55FF55,
        11 => 0xFFFF55,
        12 => 0x5555FF,
        13 => 0xFF55FF,
        14 => 0x55FFFF,
        15 => 0xFFFFFF,
        16..=231 => {
            let n = code - 16;
            let b = (n % 6) * 51;
            let g = ((n / 6) % 6) * 51;
            let r = ((n / 36) % 6) * 51;
            (r << 16) | (g << 8) | b
        }
        232..=255 => {
            let gray = (code - 232) * 10 + 8;
            (gray << 16) | (gray << 8) | gray
        }
        _ => 0xFFFFFF,
    }
}
pub(crate) fn named_text_color(name: &str) -> Option<u32> {
    Some(match name.to_ascii_lowercase().as_str() {
        "black" => 0x000000,
        "dark_blue" => 0x0000AA,
        "dark_green" => 0x00AA00,
        "dark_aqua" => 0x00AAAA,
        "dark_red" => 0xAA0000,
        "dark_purple" => 0xAA00AA,
        "gold" => 0xFFAA00,
        "gray" => 0xAAAAAA,
        "dark_gray" => 0x555555,
        "blue" => 0x5555FF,
        "green" => 0x55FF55,
        "aqua" => 0x55FFFF,
        "red" => 0xFF5555,
        "light_purple" => 0xFF55FF,
        "yellow" => 0xFFFF55,
        "white" => 0xFFFFFF,
        _ => return None,
    })
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct JsonComponentStyle {
    color: Option<u32>,
    bold: bool,
    italic: bool,
    underlined: bool,
    strikethrough: bool,
    obfuscated: bool,
}
impl JsonComponentStyle {
    fn apply_fields(self, map: &serde_json::Map<String, serde_json::Value>) -> Self {
        let mut style = self;
        if let Some(color) = map.get("color").and_then(|c| c.as_str()) {
            let parsed = if let Some(hex) = color.strip_prefix('#') {
                u32::from_str_radix(hex, 16).ok()
            } else {
                named_text_color(color)
            };
            if parsed.is_some() {
                style.color = parsed;
            }
        }
        for (key, target) in [
            ("bold", &mut style.bold),
            ("italic", &mut style.italic),
            ("underlined", &mut style.underlined),
            ("strikethrough", &mut style.strikethrough),
            ("obfuscated", &mut style.obfuscated),
        ] {
            if let Some(value) = map.get(key).and_then(|value| value.as_bool()) {
                *target = value;
            }
        }
        style
    }
}
fn push_json_style_transition(from: JsonComponentStyle, to: JsonComponentStyle, out: &mut String) {
    if from.color != to.color {
        match to.color {
            Some(rgb) => out.push_str(&format!(
                "\x1b[38;2;{};{};{}m",
                (rgb >> 16) & 0xFF,
                (rgb >> 8) & 0xFF,
                rgb & 0xFF
            )),
            None => out.push_str("\x1b[39m"),
        }
    }
    for (from_on, to_on, on, off) in [
        (from.bold, to.bold, "\x1b[1m", "\x1b[22m"),
        (from.italic, to.italic, "\x1b[3m", "\x1b[23m"),
        (from.underlined, to.underlined, "\x1b[4m", "\x1b[24m"),
        (from.strikethrough, to.strikethrough, "\x1b[9m", "\x1b[29m"),
        (from.obfuscated, to.obfuscated, "\x1b[8m", "\x1b[28m"),
    ] {
        if from_on != to_on {
            out.push_str(if to_on { on } else { off });
        }
    }
}
pub(crate) fn json_component_to_ansi(val: &serde_json::Value, out: &mut String) {
    json_component_to_ansi_with_style(val, JsonComponentStyle::default(), out);
}
fn json_component_to_ansi_with_style(
    val: &serde_json::Value,
    inherited: JsonComponentStyle,
    out: &mut String,
) {
    match val {
        serde_json::Value::String(s) => {
            out.push_str(s);
        }
        serde_json::Value::Array(arr) => {
            for elem in arr {
                json_component_to_ansi_with_style(elem, inherited, out);
            }
        }
        serde_json::Value::Object(map) => {
            let style = inherited.apply_fields(map);
            push_json_style_transition(inherited, style, out);
            if let Some(text_str) = map.get("text").and_then(|t| t.as_str()) {
                out.push_str(text_str);
            } else if let Some(fallback) = map.get("fallback").and_then(|f| f.as_str()) {
                out.push_str(fallback);
            } else if let Some(translate) = map.get("translate").and_then(|t| t.as_str()) {
                out.push_str(translate);
            }
            if let Some(extra_val) = map.get("extra") {
                json_component_to_ansi_with_style(extra_val, style, out);
            }
            push_json_style_transition(style, inherited, out);
        }
        _ => {}
    }
}
fn parse_ansi_chars_with_click_spans(
    text: &str,
    click_spans: &[ChatClickSpan],
    source: GuiMessageSource,
    content_is_json_component: bool,
) -> Vec<StyledChar> {
    let trimmed = text.trim();
    let json_expanded: String;
    let effective_text = if content_is_json_component
        && ((trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']')))
    {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
            let mut out = String::new();
            json_component_to_ansi(&val, &mut out);
            json_expanded = out;
            json_expanded.as_str()
        } else {
            text
        }
    } else {
        text
    };
    let mut output = Vec::with_capacity(effective_text.len());
    let mut style = TextStyle::default();
    let mut visible_char_index = 0usize;
    let mut byte_offset = 0;
    let bytes = effective_text.as_bytes();
    while byte_offset < bytes.len() {
        if bytes[byte_offset] == 0x1b && bytes.get(byte_offset + 1) == Some(&b'[') {
            if let Some(rel_m) = bytes[byte_offset + 2..].iter().position(|&b| b == b'm') {
                let code_start = byte_offset + 2;
                let code_end = code_start + rel_m;
                if let Ok(codes_str) = std::str::from_utf8(&bytes[code_start..code_end]) {
                    apply_ansi_codes(codes_str, &mut style);
                }
                byte_offset = code_end + 1;
                continue;
            } else {
                byte_offset += 1;
                continue;
            }
        }
        if bytes[byte_offset] == 0xc2 && bytes.get(byte_offset + 1) == Some(&0xa7) {
            let after_section = byte_offset + 2;
            if after_section < bytes.len() {
                let remaining = &effective_text[after_section..];
                if let Some(code) = remaining.chars().next() {
                    let _ = apply_legacy_code(code, &mut style);
                    byte_offset = after_section + code.len_utf8();
                    continue;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let remaining = &effective_text[byte_offset..];
        if let Some(ch) = remaining.chars().next() {
            output.push(StyledChar {
                ch,
                style,
                interaction: chat_interaction_at(click_spans, visible_char_index, source),
            });
            visible_char_index = visible_char_index.saturating_add(1);
            byte_offset += ch.len_utf8();
        } else {
            break;
        }
    }
    output
}
fn chat_interaction_at(
    click_spans: &[ChatClickSpan],
    visible_char_index: usize,
    source: GuiMessageSource,
) -> Option<ChatClickInteraction> {
    let span = click_spans.iter().find(|span| {
        span.start <= visible_char_index
            && visible_char_index < span.end
            && (span.action.is_some() || span.hover.is_some() || span.insertion.is_some())
    })?;
    Some(ChatClickInteraction {
        action: span.action.clone(),
        hover: span.hover.clone(),
        insertion: span.insertion.clone(),
        source,
    })
}
fn apply_ansi_codes(codes: &str, style: &mut TextStyle) {
    if codes.is_empty() {
        *style = TextStyle::default();
        return;
    }
    let mut values = [0u32; 16];
    let mut count = 0;
    for part in codes.split(';') {
        if let Ok(value) = part.parse::<u32>() {
            if count < values.len() {
                values[count] = value;
                count += 1;
            }
        }
    }
    let values = &values[..count];
    let mut index = 0;
    while index < values.len() {
        match values[index] {
            0 => *style = TextStyle::default(),
            1 => style.bold = true,
            3 => style.italic = true,
            4 => style.underlined = true,
            8 => style.obfuscated = true,
            9 => style.strikethrough = true,
            22 => style.bold = false,
            23 => style.italic = false,
            24 => style.underlined = false,
            28 => style.obfuscated = false,
            29 => style.strikethrough = false,
            30 => style.color = 0x000000,
            31 => style.color = 0xAA0000,
            32 => style.color = 0x00AA00,
            33 => style.color = 0xFFAA00,
            34 => style.color = 0x0000AA,
            35 => style.color = 0xAA00AA,
            36 => style.color = 0x00AAAA,
            37 => style.color = 0xAAAAAA,
            39 => style.color = TextStyle::default().color,
            90 => style.color = 0x555555,
            91 => style.color = 0xFF5555,
            92 => style.color = 0x55FF55,
            93 => style.color = 0xFFFF55,
            94 => style.color = 0x5555FF,
            95 => style.color = 0xFF55FF,
            96 => style.color = 0x55FFFF,
            97 => style.color = 0xFFFFFF,
            38 if values.get(index + 1) == Some(&2) && index + 4 < values.len() => {
                style.color = (values[index + 2].min(255) << 16)
                    | (values[index + 3].min(255) << 8)
                    | values[index + 4].min(255);
                index += 4;
            }
            38 if values.get(index + 1) == Some(&5) && index + 2 < values.len() => {
                style.color = ansi_256_to_rgb(values[index + 2]);
                index += 2;
            }
            _ => {}
        }
        index += 1;
    }
}
fn apply_legacy_code(code: char, style: &mut TextStyle) -> bool {
    match code.to_ascii_lowercase() {
        '0'..='9' | 'a'..='f' => {
            let color_code = code.to_ascii_lowercase();
            let color_id = match color_code {
                '0'..='9' => code as i32 - '0' as i32,
                'a'..='f' => 10 + (color_code as i32 - 'a' as i32),
                _ => unreachable!(),
            };
            if let Some(color) = crate::ui::scoreboard::chat_color_rgb(color_id) {
                style.color = color;
            }
            style.bold = false;
            style.italic = false;
            style.underlined = false;
            style.strikethrough = false;
            style.obfuscated = false;
            true
        }
        'k' => {
            style.obfuscated = true;
            true
        }
        'l' => {
            style.bold = true;
            true
        }
        'm' => {
            style.strikethrough = true;
            true
        }
        'n' => {
            style.underlined = true;
            true
        }
        'o' => {
            style.italic = true;
            true
        }
        'r' => {
            *style = TextStyle::default();
            true
        }
        _ => false,
    }
}
fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut byte_offset = 0;
    let bytes = text.as_bytes();
    while byte_offset < bytes.len() {
        if bytes[byte_offset] == 0x1b && bytes.get(byte_offset + 1) == Some(&b'[') {
            if let Some(rel_m) = bytes[byte_offset + 2..].iter().position(|&b| b == b'm') {
                byte_offset = byte_offset + 2 + rel_m + 1;
                continue;
            } else {
                byte_offset += 1;
                continue;
            }
        }
        if bytes[byte_offset] == 0xc2 && bytes.get(byte_offset + 1) == Some(&0xa7) {
            let after_section = byte_offset + 2;
            if after_section < bytes.len() {
                let remaining = &text[after_section..];
                if let Some(code) = remaining.chars().next() {
                    byte_offset = after_section + code.len_utf8();
                    continue;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let remaining = &text[byte_offset..];
        if let Some(ch) = remaining.chars().next() {
            out.push(ch);
            byte_offset += ch.len_utf8();
        } else {
            break;
        }
    }
    out
}
pub(crate) fn plain_chat_text(text: &str) -> String {
    strip_ansi(text)
}
pub(crate) fn formatted_chat_width(font: &FontRenderer, text: &str, scale: f32) -> f32 {
    let mut width = 0.0;
    let mut bold = false;
    let mut byte_offset = 0;
    let bytes = text.as_bytes();
    while byte_offset < bytes.len() {
        if bytes[byte_offset] == 0x1b && bytes.get(byte_offset + 1) == Some(&b'[') {
            if let Some(rel_m) = bytes[byte_offset + 2..].iter().position(|&b| b == b'm') {
                let code_start = byte_offset + 2;
                let code_end = code_start + rel_m;
                if let Ok(codes_str) = std::str::from_utf8(&bytes[code_start..code_end]) {
                    for part in codes_str.split(';') {
                        match part.parse::<u32>() {
                            Ok(0) | Ok(22) => bold = false,
                            Ok(1) => bold = true,
                            _ => {}
                        }
                    }
                }
                byte_offset = code_end + 1;
                continue;
            } else {
                byte_offset += 1;
                continue;
            }
        }
        if bytes[byte_offset] == 0xc2 && bytes.get(byte_offset + 1) == Some(&0xa7) {
            let after_section = byte_offset + 2;
            if after_section < bytes.len() {
                let remaining = &text[after_section..];
                if let Some(ch) = remaining.chars().next() {
                    match ch.to_ascii_lowercase() {
                        '0'..='9' | 'a'..='f' | 'r' => bold = false,
                        'l' => bold = true,
                        _ => {}
                    }
                    byte_offset = after_section + ch.len_utf8();
                    continue;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        let remaining = &text[byte_offset..];
        if let Some(ch) = remaining.chars().next() {
            width += font.glyph(ch).advance + if bold { 1.0 } else { 0.0 };
            byte_offset += ch.len_utf8();
        } else {
            break;
        }
    }
    width * scale
}
fn styled_run_width(font: &FontRenderer, text: &str, bold: bool, scale: f32) -> f32 {
    text.chars()
        .map(|ch| font.glyph(ch).advance + if bold { 1.0 } else { 0.0 })
        .sum::<f32>()
        * scale
}
pub(crate) fn split_formatted_chat_lines(
    font: &FontRenderer,
    text: &str,
    max_width: f32,
) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    wrap_text_inner(
        font,
        text,
        &[],
        max_width.max(1.0),
        GuiMessageSource::SystemClient,
        false,
        false,
    )
    .into_iter()
    .map(|line| styled_line_to_ansi(&line))
    .collect()
}
fn styled_line_to_ansi(line: &StyledLine) -> String {
    use std::fmt::Write;
    let mut output = String::with_capacity(line.content.len() + line.runs.len() * 24);
    for run in &line.runs {
        output.push_str("\u{1b}[0m\u{1b}[");
        let [r, g, b] = rgb(run.style.color).map(|channel| (channel * 255.0).round() as u32);
        let _ = write!(output, "38;2;{r};{g};{b}");
        if run.style.bold {
            output.push_str(";1");
        }
        if run.style.italic {
            output.push_str(";3");
        }
        if run.style.underlined {
            output.push_str(";4");
        }
        if run.style.strikethrough {
            output.push_str(";9");
        }
        if run.style.obfuscated {
            output.push_str(";8");
        }
        output.push('m');
        output.push_str(&run.content);
    }
    output
}
pub(crate) fn draw_formatted_chat_text(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    text: &str,
    x: f32,
    y: f32,
    alpha: f32,
    scale: f32,
) {
    draw_formatted_chat_text_ex(renderer, font, text, x, y, alpha, scale, true);
}
pub(crate) fn draw_formatted_chat_text_ex(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    text: &str,
    x: f32,
    y: f32,
    alpha: f32,
    scale: f32,
    shadow: bool,
) {
    let line = styled_line_from_chars(&parse_ansi_chars(text));
    draw_styled_runs(
        renderer, font, &line.runs, x, y, alpha, scale, false, shadow, None,
    );
}
pub(crate) fn draw_formatted_chat_text_override_color(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    text: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: [f32; 4],
    shadow: bool,
) {
    let line = styled_line_from_chars(&parse_ansi_chars(text));
    draw_styled_runs(
        renderer,
        font,
        &line.runs,
        x,
        y,
        1.0,
        scale,
        false,
        shadow,
        Some(color),
    );
}
fn draw_styled_runs(
    renderer: &mut GuiRenderer,
    font: &FontRenderer,
    runs: &[StyledRun],
    x: f32,
    y: f32,
    alpha: f32,
    scale: f32,
    text_is_baseline: bool,
    shadow: bool,
    color_override: Option<[f32; 4]>,
) {
    let alpha = java_alpha(alpha);
    struct PreparedRun<'a> {
        text_x: f32,
        baseline_y: f32,
        rendered_content: std::borrow::Cow<'a, str>,
        color: [f32; 4],
        bold: bool,
        italic: bool,
        underlined: bool,
        strikethrough: bool,
        decoration_x: f32,
        decoration_width: f32,
    }
    let mut prepared = Vec::with_capacity(runs.len());
    let mut cursor_x = x;
    for run in runs {
        let color = match color_override {
            Some(color) => color,
            None => {
                let [r, g, b] = rgb(run.style.color);
                [r, g, b, alpha]
            }
        };
        let text_x = cursor_x;
        let rendered_content = if run.style.obfuscated {
            std::borrow::Cow::Owned(obfuscated_text(font, &run.content))
        } else {
            std::borrow::Cow::Borrowed(run.content.as_str())
        };
        let baseline_y = if text_is_baseline {
            y
        } else {
            y + font.vanilla_ascent() * scale
        };
        let width = styled_run_width(font, &rendered_content, run.style.bold, scale);
        let decoration_x = if (cursor_x - x).abs() < f32::EPSILON {
            cursor_x - scale
        } else {
            cursor_x
        };
        let decoration_width = if (cursor_x - x).abs() < f32::EPSILON {
            width + scale
        } else {
            width
        };
        cursor_x += width;
        prepared.push(PreparedRun {
            text_x,
            baseline_y,
            rendered_content,
            color,
            bold: run.style.bold,
            italic: run.style.italic,
            underlined: run.style.underlined,
            strikethrough: run.style.strikethrough,
            decoration_x,
            decoration_width,
        });
    }
    if shadow {
        for p in &prepared {
            let shadow_color = [
                p.color[0] * 0.25,
                p.color[1] * 0.25,
                p.color[2] * 0.25,
                p.color[3],
            ];
            renderer.draw_text_styled(
                font,
                &p.rendered_content,
                p.text_x + scale,
                p.baseline_y + scale,
                shadow_color,
                scale,
                p.bold,
                p.italic,
            );
            if p.underlined {
                renderer.draw_rect(
                    p.decoration_x + scale,
                    y + 9.0 * scale,
                    p.decoration_width,
                    scale.max(1.0),
                    shadow_color,
                );
            }
            if p.strikethrough {
                renderer.draw_rect(
                    p.decoration_x + scale,
                    y + 4.5 * scale,
                    p.decoration_width,
                    scale.max(1.0),
                    shadow_color,
                );
            }
        }
    }
    for p in &prepared {
        renderer.draw_text_styled(
            font,
            &p.rendered_content,
            p.text_x,
            p.baseline_y,
            p.color,
            scale,
            p.bold,
            p.italic,
        );
        if p.underlined {
            renderer.draw_rect(
                p.decoration_x,
                y + 8.0 * scale,
                p.decoration_width,
                scale.max(1.0),
                p.color,
            );
        }
        if p.strikethrough {
            renderer.draw_rect(
                p.decoration_x,
                y + 3.5 * scale,
                p.decoration_width,
                scale.max(1.0),
                p.color,
            );
        }
    }
}
fn obfuscated_text(font: &FontRenderer, text: &str) -> String {
    static RENDER_TICK: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let frame_seed = RENDER_TICK.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ^ (web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64);
    let mut output = String::with_capacity(text.len());
    for (idx, ch) in text.chars().enumerate() {
        if ch == ' ' {
            output.push(ch);
            continue;
        }
        let char_seed =
            frame_seed.wrapping_add((idx as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)) ^ (ch as u64);
        output.push(font.obfuscated_char(ch, char_seed));
    }
    output
}
fn rgb(color: u32) -> [f32; 3] {
    [
        ((color >> 16) & 0xFF) as f32 / 255.0,
        ((color >> 8) & 0xFF) as f32 / 255.0,
        (color & 0xFF) as f32 / 255.0,
    ]
}
fn java_alpha(value: f32) -> f32 {
    (value.clamp(0.0, 1.0) * 255.0).floor() / 255.0
}
fn tag_tooltip_text(font: &FontRenderer, tag: &GuiMessageTag) -> Option<String> {
    let text = tag.text.as_deref()?;
    if text.is_empty() {
        return None;
    }
    let mut parts = text.splitn(2, '\n');
    let head = parts.next().unwrap_or("");
    let translated_head = crate::ui::i18n::translate(head);
    let combined = match parts.next() {
        Some(rest) if !rest.is_empty() => format!("{translated_head}\n{rest}"),
        _ => translated_head.to_string(),
    };
    let wrapped = wrap_text(font, &combined, 210.0);
    Some(
        wrapped
            .iter()
            .map(|line| line.content.as_str())
            .collect::<Vec<_>>()
            .join("\n"),
    )
}
fn run_with_insertion_hovered(
    font: &FontRenderer,
    runs: &[StyledRun],
    pose_x: f32,
    mouse_x: f32,
    scale: f32,
) -> bool {
    let mut run_x = pose_x;
    for run in runs {
        let run_width = styled_run_width(font, &run.content, run.style.bold, scale);
        if mouse_x >= run_x && mouse_x < run_x + run_width {
            return run
                .interaction
                .as_ref()
                .and_then(|interaction| interaction.insertion.as_ref())
                .is_some();
        }
        run_x += run_width;
    }
    false
}
fn styled_runs_width(font: &FontRenderer, runs: &[StyledRun], scale: f32) -> f32 {
    runs.iter()
        .map(|run| styled_run_width(font, &run.content, run.style.bold, scale))
        .sum()
}
fn hovered_run_interaction<'a>(
    font: &FontRenderer,
    runs: &'a [StyledRun],
    pose_x: f32,
    mouse_x: f32,
    scale: f32,
) -> Option<&'a ChatClickInteraction> {
    let mut run_x = pose_x;
    for run in runs {
        let run_width = styled_run_width(font, &run.content, run.style.bold, scale);
        if mouse_x >= run_x && mouse_x < run_x + run_width {
            return run.interaction.as_ref();
        }
        run_x += run_width;
    }
    None
}
