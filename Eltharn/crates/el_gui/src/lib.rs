pub mod components;
mod formatted_text;
pub mod framework;
pub mod input;
pub mod screens;
pub mod settings;
pub use framework::{graphics, layouts, narration, navigation, pose, render_state, screen};
pub use input::events;
pub use screens::advancements_screen::{
    AdvancementCriterionProgress, AdvancementDisplayData, AdvancementFrame,
    AdvancementIconRenderer, AdvancementNodeData, AdvancementProgressData,
    AdvancementProgressSummary, AdvancementUpdateReport, AdvancementsScreen,
    AdvancementsScreenAction, AdvancementsSnapshot, AdvancementsUpdate, SeenAdvancementsAction,
};
pub use screens::code_of_conduct_screen::{CodeOfConductAction, CodeOfConductScreen};
pub use screens::confirm_screen::{
    ConfirmAction, ConfirmContext, ConfirmLinkAction, ConfirmLinkScreen, ConfirmScreen,
    PackConfirmAction, PackConfirmScreen,
};
pub use screens::confirm_screen::{FriendsListConfirmAction, FriendsListConfirmScreen};
pub use screens::connect_screen::{ConnectingAction, ConnectingScreen};
pub use screens::credits_and_attribution::{
    CreditsAndAttributionAction, CreditsAndAttributionScreen,
};
pub use screens::direct_join_server_screen::{DirectJoinServerAction, DirectJoinServerScreen};
pub use screens::disconnected_screen::{DisconnectedAction, DisconnectedScreen};
pub use screens::in_bed_chat_screen::{InBedChatAction, InBedChatScreen};
pub use screens::manage_server_screen::{
    ManageServerAction, ManageServerScreen, ResourcePackStatus,
};
pub use screens::safety_screen::{SafetyScreen, SafetyScreenAction};
pub use screens::server_dialog_screen::{
    DialogAfterAction, DialogButtonModel, DialogClickAction, DialogMultilineOptions,
    ServerDialogModel, ServerDialogRequest, ServerDialogScreen,
};
pub use screens::server_links_screen::{ServerLinksScreen, ServerLinksScreenAction};
pub use screens::server_reconfig_screen::{ServerReconfigAction, ServerReconfigScreen};
pub use screens::stats_screen::{
    StatisticsAction, StatisticsCategory, StatisticsCell, StatisticsColumn, StatisticsData,
    StatisticsFocus, StatisticsGroup, StatisticsRow, StatsScreen,
};
pub use screens::telemetry_info_screen::{
    TELEMETRY_EVENTS, TelemetryEventDescriptor, TelemetryInfoAction, TelemetryInfoButton,
    TelemetryInfoCapabilities, TelemetryInfoScreen, TelemetryPropertyScope,
};
pub use screens::warning_screen::WarningScreen;
pub use screens::win_screen::{
    WinLine, WinScreen, WinScreenAction, WinScreenLoadError, WinScreenResources, WinTextSpan,
};
pub use screens::{
    code_of_conduct_screen, confirm_screen, connect_screen, credits_and_attribution,
    direct_join_server_screen, disconnected_screen, in_bed_chat_screen, manage_server_screen,
    multiplayer, pause_menu, safety_screen, server_dialog_screen, server_links_screen,
    server_reconfig_screen, stats_screen, telemetry_info_screen, title_screen,
};
pub use settings::accessibility_options::{AccessibilityOptionsAction, AccessibilityOptionsScreen};
pub use settings::chat_options::{ChatOptionsAction, ChatOptionsScreen};
pub use settings::controls::{ControlsAction, ControlsScreen};
pub use settings::game_rules::{
    GameRuleEditMetadata, GameRuleRow, GameRuleValue, GameRulesAction, GameRulesScreen,
};
pub use settings::keybinds::{KeyBindEntry, KeyBindsAction, KeyBindsScreen};
pub use settings::language::{
    FontSettingsAction, FontSettingsScreen, LanguageSelectAction, LanguageSelectScreen,
};
pub use settings::mouse_settings::{MouseSettingsAction, MouseSettingsScreen};
pub use settings::online_options::{OnlineOption, OnlineOptionsAction, OnlineOptionsScreen};
pub use settings::resource_packs::{
    PackCompatibility, PackEntry, PackSource, ResourcePacksAction, ResourcePacksScreen,
    pack_compatibility_for_formats,
};
pub use settings::restrictions::{
    GuiChatRestriction, GuiChatRestrictionAction, RestrictionsAction, RestrictionsScreen,
};
pub use settings::skin_customization::{SkinCustomizationAction, SkinCustomizationScreen};
pub use settings::sound_options::{SoundOptionsAction, SoundOptionsScreen, SoundOutputChoice};
pub use settings::world_options::{WorldOptionsAction, WorldOptionsScreen};
pub use settings::{
    accessibility_options, chat_options, controls, game_rules, keybinds, language, mouse_settings,
    online_options, options, resource_packs, restrictions, skin_customization, sound_options,
    video_settings, world_options,
};
pub fn is_valid_address(input: &str) -> bool {
    if input.is_empty() {
        return false;
    }
    if input.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return false;
    }
    let (host, port_str): (&str, Option<&str>) = if let Some(stripped) = input.strip_prefix('[') {
        let Some(close_bracket_idx) = stripped.find(']') else {
            return false;
        };
        let host = &stripped[..close_bracket_idx];
        if host.contains('[') {
            return false;
        }
        let rest = &stripped[close_bracket_idx + 1..];
        if rest.is_empty() {
            (host, None)
        } else {
            let Some(port) = rest.strip_prefix(':') else {
                return false;
            };
            (host, Some(port))
        }
    } else {
        match input.split_once(':') {
            Some((host, port)) => {
                if port.contains(':') {
                    return false;
                }
                (host, Some(port))
            }
            None => (input, None),
        }
    };
    if host.is_empty() {
        return false;
    }
    if let Some(port) = port_str {
        if port.is_empty() || !port.chars().all(|c| c.is_ascii_digit()) {
            return false;
        }
        if port.parse::<u16>().is_err() {
            return false;
        }
    }
    true
}
pub fn wrap_text(text: &str, max_width: f32, font: &dyn FontWidthMeasurer) -> Vec<String> {
    if formatted_text::has_formatting(text) {
        return formatted_text::wrap(text, max_width, font);
    }
    let max_width = max_width.max(1.0);
    let mut lines = Vec::new();
    let mut chars: Vec<char> = Vec::new();
    for paragraph in text.split('\n') {
        chars.clear();
        chars.extend(paragraph.chars());
        let mut start = 0usize;
        loop {
            if start >= chars.len() {
                lines.push(String::new());
                break;
            }
            let mut width = 0.0f32;
            let mut had_non_zero_width_char = false;
            let mut last_space: Option<usize> = None;
            let mut next_char = start;
            let mut line_break: Option<usize> = None;
            for i in start..chars.len() {
                let codepoint = chars[i];
                if codepoint == ' ' {
                    last_space = Some(i);
                }
                let char_width = font.measure_char(codepoint, 1.0);
                width += char_width;
                if !had_non_zero_width_char || !(width > max_width) {
                    if char_width != 0.0 {
                        had_non_zero_width_char = true;
                    }
                    next_char = i + 1;
                } else if let Some(space) = last_space {
                    line_break = Some(space);
                } else {
                    line_break = Some(i);
                }
                if line_break.is_some() {
                    break;
                }
            }
            let end = line_break.unwrap_or(next_char);
            let line: String = chars[start..end].iter().collect();
            if line.is_empty() {
                lines.push(String::new());
                start = end;
            } else {
                lines.push(line);
                let break_char = chars.get(end).copied();
                let adjust = match break_char {
                    Some(' ') | Some('\n') => 1,
                    _ => 0,
                };
                start = end + adjust;
                if line_break.is_none() && (end == next_char || end == chars.len()) {
                    break;
                }
            }
        }
    }
    lines
}
pub use components::{
    Button, GuiEventListener, Label, Renderable, TabOrderedElement, Widget, WidgetBase,
    WidgetClickSound,
    checkbox::Checkbox,
    cycle_button::CycleButton,
    scroll_area::{AbstractScrollArea, ScrollableLayout, ScrollbarSettings},
    selection_list::{ContainerObjectSelectionList, ObjectSelectionList, SelectionListEntry},
    slider_button::SliderButton,
    sprites::{WidgetSprites, sprites, widget_sprite},
    text_widget::{
        FittingMultiLineTextWidget, MultiLineLabel, MultiLineTextWidget, StringWidget,
        TextAlignment, TextOverflow,
    },
};
pub use framework::focus::{
    java_border, resolve_arrow, resolve_arrow_from, resolve_initial_focus, resolve_path,
    resolve_tab, widget_accepts_navigation,
};
pub use framework::graphics::GuiGraphicsExtractor;
pub use framework::layouts::{
    CommonLayouts, EqualSpacingLayout, FrameLayout, GridLayout, HeaderAndFooterLayout, Layout,
    LayoutElement, LayoutSettings, LinearLayout, Orientation, SpacerElement,
};
pub use framework::narration::{
    NarratableEntry, NarratedElementType, NarrationElementOutput, NarrationPriority,
};
pub use framework::navigation::{
    ComponentPath, FocusNavigationEvent, ScreenAxis, ScreenDirection, ScreenPosition,
    ScreenRectangle,
};
pub use framework::render_state::{
    GuiElementRenderState, GuiRenderState, GuiTextRenderState, RenderPipeline, TextureSetup,
    TraverseRange,
};
pub use framework::screen::{
    Screen, ScreenAction, ScreenBase, ScreenStack, screen_added, screen_char_typed,
    screen_fill_crash_details, screen_handle_delayed_narration, screen_init, screen_key_pressed,
    screen_key_released, screen_mouse_clicked, screen_mouse_dragged, screen_mouse_moved,
    screen_mouse_released, screen_mouse_scrolled, screen_narration_message, screen_preedit_updated,
    screen_rebuild_widgets, screen_removed, screen_resize, screen_set_initial_focus, screen_tick,
    screen_update_narration, screen_usage_narration,
};
pub use input::events::{
    CharacterEvent, CursorType, KeyEvent, MouseButtonEvent, MouseButtonInfo, PreeditEvent,
};
use std::sync::OnceLock;
static TRANSLATOR: OnceLock<fn(&str) -> String> = OnceLock::new();
pub fn register_translator(translator: fn(&str) -> String) {
    let _ = TRANSLATOR.set(translator);
}
pub fn translate(key: &str) -> String {
    let text = if let Some(translator) = TRANSLATOR.get() {
        translator(key)
    } else {
        key.to_string()
    };
    if text.contains("%%") {
        text.replace("%%", "%")
    } else {
        text
    }
}
pub trait FontWidthMeasurer {
    fn measure_width(&self, text: &str, scale: f32) -> f32;
    fn measure_char(&self, ch: char, scale: f32) -> f32 {
        let mut buf = [0u8; 4];
        let s = ch.encode_utf8(&mut buf);
        self.measure_width(s, scale)
    }
}
impl<T: FontWidthMeasurer + ?Sized> FontWidthMeasurer for std::sync::Arc<T> {
    fn measure_width(&self, text: &str, scale: f32) -> f32 {
        T::measure_width(self, text, scale)
    }
}
