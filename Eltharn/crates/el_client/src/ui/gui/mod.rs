pub mod autoshot;
mod extension_menus;
pub mod font;
pub mod hud;
pub mod item_renderer;
mod menu_data;
pub mod realms_texture_manager;
pub mod renderer;
pub mod scoreboard_hud;
pub mod screen;
mod screen_events;
pub mod screenshot;
use crate::app::main_entry::ProtocolBackendData;
use crate::client::waypoint::LocatorBarMarker;
use crate::gameplay::camera::CameraType;
use crate::gameplay::game_mode::GameType;
use crate::gameplay::player::LocalPlayer;
#[cfg(feature = "extensions")]
mod mod_app;
#[cfg(feature = "extensions")]
mod mod_content;
use crate::network::protocol_backend::{self, ClientConnectRequest, RemoteConnectionHandle};
use crate::render::profiler::RenderProfiler;
use crate::resources::manager::{ClientResourceManager, ResourceError};
use crate::ui::domain::ChatClickAction;
use crate::ui::domain::{ChatCompletionAction, NetworkChatSuggestion, PlayerInfoUpdateEntry};
use crate::ui::gui::font::{FontOptions, FontRenderer};
use crate::ui::gui::hud::HudRenderer;
use crate::ui::gui::realms_texture_manager::RealmsTextureManager;
use crate::ui::gui::renderer::{DrawBatch, GuiRenderer, GuiVertex};
use crate::ui::gui::screen::InventoryEntityPreview;
use crate::world::client_level::ClientLevel;
use crate::world::client_level::WeatherMode;
use el_core::resource_location::ResourceLocation;
use el_gui::FontWidthMeasurer;
use el_gui::framework::render_state::RenderPipeline;
use el_gui::framework::screen::Screen;
use el_gui::screens::advancements_screen::{
    AdvancementDisplayData, AdvancementIconRenderer, AdvancementsScreen, AdvancementsSnapshot,
    SeenAdvancementsAction,
};
use el_gui::screens::credits_and_attribution::{
    CreditsAndAttributionAction, CreditsAndAttributionScreen,
};
use el_gui::screens::death_screen::{DeathScreen, DeathScreenAction};
use el_gui::screens::extension_catalog::{
    ExtensionCatalog, ExtensionCatalogAction, ExtensionCatalogKind, ExtensionCatalogScreen,
};
use el_gui::screens::multiplayer::{
    LanServerEntry, MultiplayerAction, MultiplayerScreen, ServerEntry,
};
use el_gui::screens::pause_menu::{PauseMenuAction, PauseMenuScreen, PauseSubmenu};
use el_gui::screens::server_dialog_screen::{
    DialogAfterAction, DialogButtonModel, DialogClickAction, DialogInputControl, DialogInputValue,
    DialogItemVisual, DialogMultilineOptions, ServerDialogBackground, ServerDialogModel,
    ServerDialogScreen, sign_editor_dialog_model,
};
use el_gui::screens::server_links_screen::{ServerLinksScreen, ServerLinksScreenAction};
use el_gui::screens::stats_screen::{StatisticsAction, StatsScreen};
use el_gui::screens::telemetry_info_screen::{
    TelemetryInfoAction, TelemetryInfoCapabilities, TelemetryInfoScreen,
};
use el_gui::screens::title_screen::{TitleScreen, TitleScreenAction, TitleScreenSettings};
use el_gui::screens::win_screen::{
    CREDITS_RESOURCE, END_POEM_RESOURCE, POSTCREDITS_RESOURCE, WinScreen, WinScreenResources,
};
use el_gui::settings::accessibility_options::{
    AccessibilityOptionsAction, AccessibilityOptionsOption, AccessibilityOptionsScreen,
};
use el_gui::settings::chat_options::ChatOptionsScreen;
use el_gui::settings::controls::{
    ControlsAction, ControlsOption, ControlsScreen, ControlsToggleOptions,
};
use el_gui::settings::game_rules::{GameRuleValue, GameRulesAction, GameRulesScreen};
use el_gui::settings::keybinds::{KeyBindsAction, KeyBindsScreen};
use el_gui::settings::language::{
    FontSettingsAction, FontSettingsScreen, LanguageSelectAction, LanguageSelectScreen,
};
use el_gui::settings::mouse_settings::MouseSettingsScreen;
use el_gui::settings::online_options::{OnlineOptionsAction, OnlineOptionsScreen};
use el_gui::settings::options::{
    OptionsAction, OptionsScreen, fov_slider_from_java_option, java_fov_option_from_slider,
};
use el_gui::settings::resource_packs::{PackEntry, ResourcePacksAction, ResourcePacksScreen};
use el_gui::settings::restrictions::{
    GuiChatRestriction, GuiChatRestrictionAction, RestrictionsAction, RestrictionsScreen,
};
use el_gui::settings::skin_customization::{SkinCustomizationAction, SkinCustomizationScreen};
use el_gui::settings::sound_options::{SoundOptionsAction, SoundOptionsScreen};
use el_gui::settings::video_settings::{
    FullscreenVideoMode, VideoSettingsAction, VideoSettingsScreen,
    ambient_occlusion_enabled_from_options,
};
use el_gui::settings::world_options::{WorldOptionsAction, WorldOptionsScreen};
use el_gui::{
    CodeOfConductAction, CodeOfConductScreen, ConfirmAction, ConfirmContext, ConfirmLinkAction,
    ConfirmLinkScreen, ConfirmScreen, ConnectingAction, ConnectingScreen, DirectJoinServerAction,
    DirectJoinServerScreen, DisconnectedAction, DisconnectedScreen, FriendsListConfirmAction,
    FriendsListConfirmScreen, InBedChatAction, InBedChatScreen, ManageServerAction,
    ManageServerScreen, PackConfirmAction, PackConfirmScreen, ResourcePackStatus, SafetyScreen,
    SafetyScreenAction, ServerReconfigAction, ServerReconfigScreen,
};
use el_gui::{
    GuiElementRenderState, GuiGraphicsExtractor, GuiRenderState, GuiTextRenderState, TraverseRange,
};
use el_inventory::container::Container;
use el_protocol::packets::game::{
    ServerboundContainerClickPacket, ServerboundSetCreativeModeSlotPacket,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use wgpu::{Device, Queue, RenderPass};
use winit::event::{ElementState, Ime, MouseButton, WindowEvent};
use winit::keyboard::{KeyCode, PhysicalKey};
const SERVER_ROW_HEIGHT_PX: usize = 36;
fn chat_parity_probe(detail: impl std::fmt::Display) {
    el_protocol_backend::diagnostics::write_line(&format!("[ParityProbe] chat {detail}"));
}
fn ui_crash_trace(detail: impl std::fmt::Display) {
    el_protocol_backend::diagnostics::write_line(&format!("[UiCrashTrace] {detail}"));
}
fn dialog_type_path(value: &serde_json::Value) -> Option<&str> {
    value
        .get("type")
        .and_then(serde_json::Value::as_str)
        .map(|kind| kind.strip_prefix("minecraft:").unwrap_or(kind))
}
fn dialog_component_text(value: &serde_json::Value) -> String {
    crate::ui::gui::screen::container::styled_text_component(value)
}
fn should_refresh_now_playing_toast_after_volume_change(
    source: crate::client::sound_engine::SoundSource,
    final_music_volume: f32,
) -> bool {
    matches!(
        source,
        crate::client::sound_engine::SoundSource::Master
            | crate::client::sound_engine::SoundSource::Music
    ) && final_music_volume > 0.0
}
fn should_start_now_playing_toast_for_new_preference(
    previous: Option<&str>,
    next: &str,
    has_current_music: bool,
    final_music_volume: f32,
) -> bool {
    next == "pause_and_toast"
        && previous != Some("pause_and_toast")
        && has_current_music
        && final_music_volume > 0.0
}
fn parse_dialog_click_action(
    value: &serde_json::Value,
) -> Result<Option<DialogClickAction>, &'static str> {
    let kind = dialog_type_path(value).ok_or("dialog action has no type")?;
    let string = |field: &str| value.get(field).and_then(serde_json::Value::as_str);
    let action = match kind {
        "run_command" => DialogClickAction::RunCommand {
            command: string("command")
                .ok_or("command action has no command")?
                .to_string(),
            dynamic: false,
        },
        "dynamic/run_command" => {
            let template = string("template").ok_or("dynamic command action has no template")?;
            validate_dialog_template(template)?;
            DialogClickAction::RunCommand {
                command: template.to_string(),
                dynamic: true,
            }
        }
        "open_url" => {
            DialogClickAction::OpenUrl(string("url").ok_or("URL action has no URL")?.to_string())
        }
        "copy_to_clipboard" => DialogClickAction::CopyToClipboard(
            string("value")
                .ok_or("clipboard action has no value")?
                .to_string(),
        ),
        "suggest_command" => DialogClickAction::SuggestCommand(
            string("command")
                .ok_or("suggest-command action has no command")?
                .to_string(),
        ),
        "custom" => DialogClickAction::Custom {
            id: string("id")
                .ok_or("custom action has no identifier")?
                .to_string(),
            payload: value
                .get("payload")
                .filter(|payload| !payload.is_null())
                .map(dialog_nbt_wire_payload)
                .transpose()?,
            dynamic_inputs: false,
        },
        "dynamic/custom" => DialogClickAction::Custom {
            id: string("id")
                .ok_or("dynamic custom action has no identifier")?
                .to_string(),
            payload: value
                .get("additions")
                .filter(|additions| !additions.is_null())
                .map(dialog_nbt_wire_payload)
                .transpose()?,
            dynamic_inputs: true,
        },
        _ => return Err("dialog action type is not supported by this client"),
    };
    Ok(Some(action))
}
fn dialog_nbt_wire_payload(value: &serde_json::Value) -> Result<Vec<u8>, &'static str> {
    const MAX_NBT_TAG_BYTES: usize = 65_536;
    let Some(encoded) = value
        .get("__eltharn_nbt_wire")
        .and_then(serde_json::Value::as_str)
    else {
        return Err("custom click payload was not preserved by the protocol backend");
    };
    let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
        .map_err(|_| "custom click payload metadata is malformed")?;
    if bytes.is_empty() || bytes.len() > MAX_NBT_TAG_BYTES {
        return Err("custom click payload exceeds the serverbound NBT limit");
    }
    Ok(bytes)
}
fn validate_dialog_template(template: &str) -> Result<(), &'static str> {
    let bytes = template.as_bytes();
    let mut cursor = 0;
    let mut found_variable = false;
    while cursor < bytes.len() {
        if bytes[cursor] == b'$' && bytes.get(cursor + 1) == Some(&b'(') {
            let start = cursor + 2;
            let Some(relative_end) = bytes[start..].iter().position(|byte| *byte == b')') else {
                return Err("dynamic command template has an unterminated variable");
            };
            let end = start + relative_end;
            if !template[start..end]
                .chars()
                .all(|character| character.is_alphanumeric() || character == '_')
            {
                return Err("dynamic command template has an invalid input key");
            }
            found_variable = true;
            cursor = end + 1;
        } else {
            cursor += 1;
        }
    }
    if !found_variable {
        return Err("dynamic command template has no input variables");
    }
    Ok(())
}
fn parse_dialog_inputs(data: &serde_json::Value) -> Result<Vec<DialogInputControl>, &'static str> {
    let Some(inputs) = data.get("inputs").and_then(serde_json::Value::as_array) else {
        return Ok(Vec::new());
    };
    if inputs.len() > 32 {
        return Err("dialog has more than 32 input controls");
    }
    let mut keys = std::collections::HashSet::new();
    let mut parsed = Vec::with_capacity(inputs.len());
    for input in inputs {
        let key = input
            .get("key")
            .and_then(serde_json::Value::as_str)
            .filter(|key| {
                key.chars()
                    .all(|character| character.is_alphanumeric() || character == '_')
            })
            .ok_or("dialog input has an invalid key")?;
        if !keys.insert(key.to_string()) {
            return Err("dialog has duplicate input keys");
        }
        let control = input.get("control").unwrap_or(input);
        let kind =
            dialog_type_path(control).ok_or("dialog input has no registered control type")?;
        let string = |field: &str| control.get(field).and_then(serde_json::Value::as_str);
        let label = control
            .get("label")
            .map(dialog_component_text)
            .filter(|label| !label.is_empty())
            .ok_or("dialog input has no label")?;
        let control = match kind {
            "text" => {
                let initial = string("initial").unwrap_or_default().to_string();
                let max_length = control
                    .get("max_length")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(32)
                    .clamp(1, 32_767) as usize;
                if initial.encode_utf16().count() > max_length {
                    return Err("dialog text input default exceeds its maximum length");
                }
                let width = control
                    .get("width")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(200)
                    .clamp(1, 4096) as i32;
                let label_visible = control
                    .get("label_visible")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(true);
                let multiline = control
                    .get("multiline")
                    .filter(|value| !value.is_null())
                    .map(|options| {
                        let max_lines = options
                            .get("max_lines")
                            .and_then(serde_json::Value::as_u64)
                            .map(|lines| lines.clamp(1, 512) as usize);
                        let height = options
                            .get("height")
                            .and_then(serde_json::Value::as_u64)
                            .map(|height| height.clamp(1, 512) as i32)
                            .unwrap_or_else(|| 9 * max_lines.unwrap_or(4) as i32 + 8)
                            .min(512);
                        DialogMultilineOptions { max_lines, height }
                    });
                DialogInputControl::Text {
                    key: key.to_string(),
                    label,
                    initial,
                    max_length,
                    width,
                    label_visible,
                    multiline,
                }
            }
            "boolean" => DialogInputControl::Boolean {
                key: key.to_string(),
                label,
                value: control
                    .get("initial")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
                on_true: string("on_true").unwrap_or("true").to_string(),
                on_false: string("on_false").unwrap_or("false").to_string(),
            },
            "number_range" => {
                let range = control
                    .get("range_info")
                    .ok_or("number input has no range_info")?;
                let start = range
                    .get("start")
                    .and_then(serde_json::Value::as_f64)
                    .filter(|value| value.is_finite())
                    .ok_or("number input has an invalid start")? as f32;
                let end = range
                    .get("end")
                    .and_then(serde_json::Value::as_f64)
                    .filter(|value| value.is_finite())
                    .ok_or("number input has an invalid end")? as f32;
                let initial = range
                    .get("initial")
                    .and_then(serde_json::Value::as_f64)
                    .filter(|value| value.is_finite())
                    .map(|value| value as f32)
                    .unwrap_or((start + end) / 2.0);
                let minimum = start.min(end);
                let maximum = start.max(end);
                if !start.is_finite()
                    || !end.is_finite()
                    || !initial.is_finite()
                    || initial < minimum
                    || initial > maximum
                {
                    return Err("number input has an invalid range or initial value");
                }
                let step = range
                    .get("step")
                    .and_then(serde_json::Value::as_f64)
                    .filter(|value| value.is_finite() && *value > 0.0)
                    .map(|value| value as f32);
                DialogInputControl::NumberRange {
                    key: key.to_string(),
                    label,
                    start,
                    end,
                    value: initial,
                    step,
                }
            }
            "single_option" => {
                let entries = control
                    .get("options")
                    .and_then(serde_json::Value::as_array)
                    .filter(|entries| !entries.is_empty() && entries.len() <= 128)
                    .ok_or("single-option input has no bounded option list")?;
                let mut options = Vec::with_capacity(entries.len());
                let mut selected = None;
                for (index, entry) in entries.iter().enumerate() {
                    let (id, display, initial) = if let Some(id) = entry.as_str() {
                        (id.to_string(), id.to_string(), false)
                    } else {
                        let id = entry
                            .get("id")
                            .and_then(serde_json::Value::as_str)
                            .ok_or("single-option entry has no id")?
                            .to_string();
                        let display = entry
                            .get("display")
                            .map(dialog_component_text)
                            .filter(|value| !value.is_empty())
                            .unwrap_or_else(|| id.clone());
                        let initial = entry
                            .get("initial")
                            .and_then(serde_json::Value::as_bool)
                            .unwrap_or(false);
                        (id, display, initial)
                    };
                    if initial {
                        if selected.replace(index).is_some() {
                            return Err("single-option input has multiple initial entries");
                        }
                    }
                    options.push((id, display));
                }
                DialogInputControl::SingleOption {
                    key: key.to_string(),
                    label,
                    options,
                    selected: selected.unwrap_or(0),
                }
            }
            _ => return Err("dialog input control type is not supported"),
        };
        parsed.push(control);
    }
    Ok(parsed)
}
fn parse_dialog_action_button(
    value: &serde_json::Value,
) -> Result<DialogButtonModel, &'static str> {
    let label = value
        .get("label")
        .map(dialog_component_text)
        .filter(|label| !label.is_empty())
        .ok_or("dialog action button has no label")?;
    let width = value
        .get("width")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(150)
        .clamp(1, 1024) as i32;
    let action = value
        .get("action")
        .map(parse_dialog_click_action)
        .transpose()?
        .flatten();
    Ok(DialogButtonModel {
        label,
        width,
        action,
    })
}
fn unsupported_server_dialog(title: String, reason: &str) -> ServerDialogModel {
    ServerDialogModel {
        title,
        body: vec![format!(
            "This server dialog cannot be displayed safely: {reason}."
        )],
        body_items: Vec::new(),
        buttons: Vec::new(),
        footer_buttons: vec![DialogButtonModel {
            label: "Done".to_string(),
            width: 200,
            action: None,
        }],
        cancel_action: None,
        inputs: Vec::new(),
        can_close_with_escape: true,
        pause: true,
        after_action: DialogAfterAction::Close,
        columns: 1,
    }
}
fn server_dialog_model(
    registered_kind: Option<&str>,
    data: Option<&serde_json::Value>,
    server_links: &[(String, String)],
) -> ServerDialogModel {
    let Some(data) = data else {
        return unsupported_server_dialog(
            "Server Dialog".to_string(),
            "the backend supplied a registry reference without its resolved dialog data",
        );
    };
    let title = data
        .get("title")
        .map(dialog_component_text)
        .filter(|title| !title.is_empty())
        .unwrap_or_else(|| "Server Dialog".to_string());
    let kind = dialog_type_path(data)
        .or_else(|| registered_kind.and_then(|kind| kind.rsplit("::").next()))
        .unwrap_or_default();
    let mut body = Vec::new();
    let mut body_items = Vec::new();
    if let Some(entries) = data.get("body").and_then(serde_json::Value::as_array) {
        for entry in entries {
            match dialog_type_path(entry) {
                Some("plain_message") => {
                    if let Some(contents) = entry.get("contents") {
                        let text = dialog_component_text(contents);
                        if !text.is_empty() {
                            body.push(text);
                        }
                    }
                }
                Some("item") => {
                    if let Some(description) = entry.get("description")
                        && let Some(contents) = description.get("contents")
                    {
                        let text = dialog_component_text(contents);
                        if !text.is_empty() {
                            body.push(text);
                        }
                    }
                    let item_template = entry.get("item");
                    let item_id = item_template
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| {
                            item_template
                                .and_then(|item| item.get("id"))
                                .and_then(serde_json::Value::as_str)
                        });
                    if let Some(item_id) = item_id
                        && let Some(location) = ResourceLocation::try_from_string(item_id)
                    {
                        let item = el_core::registry::BuiltInRegistries::items()
                            .get_by_location(&location);
                        if item.location == location {
                            let count = item_template
                                .and_then(|item| item.get("count"))
                                .and_then(serde_json::Value::as_i64)
                                .unwrap_or(1)
                                .clamp(1, 99) as i32;
                            let mut stack = el_core::item::ItemStack::new(item.clone(), count);
                            if let Some(components) = item_template
                                .and_then(|item| item.get("components"))
                                .and_then(serde_json::Value::as_object)
                            {
                                if let Some(custom_name) = components.get("minecraft:custom_name") {
                                    stack.components.set(
                                        "minecraft:custom_name",
                                        el_core::item::DataComponent::CustomName(
                                            dialog_component_text(custom_name),
                                        ),
                                    );
                                }
                                if let Some(damage) = components
                                    .get("minecraft:damage")
                                    .and_then(serde_json::Value::as_i64)
                                {
                                    stack.components.set(
                                        "minecraft:damage",
                                        el_core::item::DataComponent::Damage(damage as i32),
                                    );
                                }
                            }
                            let dimension = |key: &str| {
                                entry
                                    .get(key)
                                    .and_then(serde_json::Value::as_i64)
                                    .unwrap_or(16)
                                    .clamp(1, 256) as i32
                            };
                            body_items.push(DialogItemVisual {
                                stack,
                                width: dimension("width"),
                                height: dimension("height"),
                                show_decorations: entry
                                    .get("show_decorations")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(true),
                                show_tooltip: entry
                                    .get("show_tooltip")
                                    .and_then(serde_json::Value::as_bool)
                                    .unwrap_or(true),
                            });
                        } else if body.is_empty() {
                            body.push(format!("Item: {item_id}"));
                        }
                    }
                }
                _ => {
                    return unsupported_server_dialog(
                        title,
                        "the dialog body includes a rendering type this client does not support",
                    );
                }
            }
        }
    }
    let mut buttons = Vec::new();
    let mut footer_buttons = Vec::new();
    let mut cancel_action = None;
    let columns = data
        .get("columns")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(2)
        .clamp(1, 4) as usize;
    let button_values: Vec<&serde_json::Value> = match kind {
        "notice" => {
            if let Some(value) = data.get("action") {
                let button = match parse_dialog_action_button(value) {
                    Ok(button) => button,
                    Err(reason) => return unsupported_server_dialog(title, reason),
                };
                cancel_action = button.action.clone();
                footer_buttons.push(button);
            }
            Vec::new()
        }
        "confirmation" => {
            for name in ["yes", "no"] {
                let Some(value) = data.get(name) else {
                    return unsupported_server_dialog(
                        title,
                        "confirmation dialog is missing a button",
                    );
                };
                let button = match parse_dialog_action_button(value) {
                    Ok(button) => button,
                    Err(reason) => return unsupported_server_dialog(title, reason),
                };
                if name == "no" {
                    cancel_action = button.action.clone();
                }
                footer_buttons.push(button);
            }
            Vec::new()
        }
        "multi_action" => data
            .get("actions")
            .and_then(serde_json::Value::as_array)
            .map(|actions| actions.iter().collect())
            .unwrap_or_default(),
        "dialog_list" => {
            let Some(entries) = data.get("dialogs").and_then(serde_json::Value::as_array) else {
                return unsupported_server_dialog(
                    title,
                    "dialog list has no resolved child dialogs",
                );
            };
            for child in entries {
                let Some(child_data) = child.as_object() else {
                    return unsupported_server_dialog(
                        title,
                        "dialog list child reference was not resolved",
                    );
                };
                let child_title = child_data
                    .get("external_title")
                    .or_else(|| child_data.get("title"))
                    .map(dialog_component_text)
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| "Open dialog".to_string());
                let child_model = server_dialog_model(None, Some(child), &[]);
                if child_model.body.first().is_some_and(|line| {
                    line.starts_with("This server dialog cannot be displayed safely:")
                }) {
                    return unsupported_server_dialog(
                        title,
                        "a child dialog is not supported by this client",
                    );
                }
                buttons.push(DialogButtonModel {
                    label: child_title,
                    width: data
                        .get("button_width")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(150)
                        .clamp(1, 1024) as i32,
                    action: Some(DialogClickAction::OpenDialog(Box::new(child_model))),
                });
            }
            Vec::new()
        }
        "server_links" => {
            for (label, url) in server_links {
                buttons.push(DialogButtonModel {
                    label: el_gui::screens::server_links_screen::server_link_label(label),
                    width: data
                        .get("button_width")
                        .and_then(serde_json::Value::as_i64)
                        .unwrap_or(150)
                        .clamp(1, 1024) as i32,
                    action: Some(DialogClickAction::OpenUrl(url.clone())),
                });
            }
            Vec::new()
        }
        _ => {
            let registered_kind = registered_kind.unwrap_or("unrecognized");
            return unsupported_server_dialog(
                title,
                &format!("dialog type `{kind}` from `{registered_kind}` is not supported"),
            );
        }
    };
    for value in button_values {
        match parse_dialog_action_button(value) {
            Ok(button) => buttons.push(button),
            Err(reason) => return unsupported_server_dialog(title, reason),
        }
    }
    if let Some(exit) = data.get("exit_action") {
        match parse_dialog_action_button(exit) {
            Ok(button) => {
                cancel_action = button.action.clone();
                footer_buttons.push(button);
            }
            Err(reason) => return unsupported_server_dialog(title, reason),
        }
    }
    let after_action = match data
        .get("after_action")
        .and_then(serde_json::Value::as_str)
        .and_then(|value| value.rsplit(':').next())
        .unwrap_or("close")
    {
        "none" => DialogAfterAction::KeepOpen,
        "wait_for_response" => DialogAfterAction::WaitForResponse,
        _ => DialogAfterAction::Close,
    };
    let inputs = match parse_dialog_inputs(data) {
        Ok(inputs) => inputs,
        Err(reason) => return unsupported_server_dialog(title, reason),
    };
    ServerDialogModel {
        title,
        body,
        body_items,
        buttons,
        footer_buttons,
        cancel_action,
        inputs,
        can_close_with_escape: data
            .get("can_close_with_escape")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        pause: data
            .get("pause")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        after_action,
        columns,
    }
}
fn instantiate_dialog_template(
    template: &str,
    inputs: &std::collections::HashMap<String, DialogInputValue>,
) -> Option<String> {
    const MAX_COMMAND_LENGTH: usize = 32_767;
    let mut output = String::with_capacity(template.len());
    let mut cursor = 0;
    while let Some(relative_start) = template[cursor..].find("$(") {
        let start = cursor + relative_start;
        output.push_str(&template[cursor..start]);
        let variable_start = start + 2;
        let relative_end = template[variable_start..].find(')')?;
        let end = variable_start + relative_end;
        let name = &template[variable_start..end];
        if let Some(value) = inputs.get(name) {
            output.push_str(&dialog_template_value(value));
        }
        if output.len() > MAX_COMMAND_LENGTH {
            return None;
        }
        cursor = end + 1;
    }
    output.push_str(&template[cursor..]);
    (output.len() <= MAX_COMMAND_LENGTH).then_some(output)
}
fn build_dialog_custom_payload(
    base_payload: Option<&[u8]>,
    dynamic_inputs: bool,
    inputs: &std::collections::HashMap<String, DialogInputValue>,
) -> Result<Option<Vec<u8>>, &'static str> {
    const MAX_NBT_TAG_BYTES: usize = 65_536;
    if !dynamic_inputs {
        return Ok(base_payload.map(ToOwned::to_owned));
    }
    let mut tag = match base_payload {
        Some(bytes) => el_nbt::io::read_any_compound(bytes)
            .map_err(|_| "dynamic custom additions are invalid NBT")?,
        None => el_nbt::Tag::Compound(Vec::new()),
    };
    let el_nbt::Tag::Compound(fields) = &mut tag else {
        return Err("dynamic custom additions are not an NBT compound");
    };
    for (key, value) in inputs {
        let value = match value {
            DialogInputValue::Text(value) | DialogInputValue::Option(value) => {
                el_nbt::Tag::String(value.clone())
            }
            DialogInputValue::Boolean { value, .. } => el_nbt::Tag::Byte(i8::from(*value)),
            DialogInputValue::Number(value) if value.is_finite() => el_nbt::Tag::Float(*value),
            DialogInputValue::Number(_) => {
                return Err("dynamic custom input contains a non-finite number");
            }
        };
        if let Some((_, existing)) = fields.iter_mut().find(|(name, _)| name == key) {
            *existing = value;
        } else {
            fields.push((key.clone(), value));
        }
    }
    let mut payload = vec![tag.get_id()];
    el_nbt::io::write_payload(&mut payload, &tag)
        .map_err(|_| "dynamic custom input could not be encoded as NBT")?;
    if payload.len() > MAX_NBT_TAG_BYTES {
        return Err("dynamic custom payload exceeds the serverbound NBT limit");
    }
    Ok(Some(payload))
}
fn dialog_template_value(value: &DialogInputValue) -> String {
    match value {
        DialogInputValue::Text(value) => escape_dialog_template_string(value),
        DialogInputValue::Boolean { substitution, .. } => substitution.clone(),
        DialogInputValue::Number(value) => {
            if value.is_finite() && value.fract() == 0.0 {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        DialogInputValue::Option(value) => value.clone(),
    }
}
fn escape_dialog_template_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' | '\'' | '\\' => {
                escaped.push('\\');
                escaped.push(character);
            }
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            '\u{0008}' => escaped.push_str("\\b"),
            '\u{000C}' => escaped.push_str("\\f"),
            c if c < '\u{0020}' => {
                use std::fmt::Write as _;
                let _ = write!(escaped, "\\x{:02x}", c as u32);
            }
            c => escaped.push(c),
        }
    }
    escaped
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeRequest {
    Set(i64),
    Add(i64),
}
pub enum ActiveScreen {
    Title(TitleScreen),
    NavigationTransition,
    Pause(PauseMenuScreen),
    Options(OptionsScreen),
    Video(VideoSettingsScreen),
    SkinCustomization(SkinCustomizationScreen),
    ChatOptions(ChatOptionsScreen),
    SoundOptions(SoundOptionsScreen),
    OnlineOptions(OnlineOptionsScreen),
    TelemetryInfo(TelemetryInfoScreen),
    ServerLinks(ServerLinksScreen),
    ServerDialog {
        screen: ServerDialogScreen,
        parent: Option<Box<ActiveScreen>>,
        in_world: bool,
    },
    WorldOptions(WorldOptionsScreen),
    GameRules(GameRulesScreen),
    Controls(ControlsScreen),
    CreditsAndAttribution(CreditsAndAttributionScreen),
    WinScreen {
        screen: WinScreen,
        parent: Option<Box<ActiveScreen>>,
    },
    DeathScreen(DeathScreen),
    Stats(StatsScreen),
    Advancements(AdvancementsScreen),
    AccessibilityOptions(AccessibilityOptionsScreen),
    MouseSettings(MouseSettingsScreen),
    Restrictions(RestrictionsScreen),
    LanguageSelect(LanguageSelectScreen),
    FontSettings(FontSettingsScreen),
    KeyBinds {
        screen: KeyBindsScreen,
        waiting_for_key: Option<String>,
    },
    SubmenuStub {
        submenu: PauseSubmenu,
        in_world: bool,
    },
    Chat {
        input: String,
    },
    InBedChat(InBedChatScreen),
    Container(crate::ui::gui::screen::container::ContainerScreenState),
    Multiplayer(MultiplayerScreen),
    ManageServer(ManageServerScreen),
    DirectConnect(DirectJoinServerScreen),
    Connecting(ConnectingScreen),
    LevelLoading(LevelLoadingScreen),
    Disconnected(DisconnectedScreen),
    Safety(SafetyScreen),
    CodeOfConduct(CodeOfConductScreen),
    Confirm(ConfirmScreen),
    ConfirmLink(ConfirmLinkScreen),
    PackConfirm(PackConfirmScreen),
    FriendsConfirm {
        screen: FriendsListConfirmScreen,
        parent: Option<Box<ActiveScreen>>,
        in_world: bool,
    },
    ServerReconfig(ServerReconfigScreen),
    ResourcePacks(ResourcePacksScreen),
    ExtensionCatalog {
        screen: ExtensionCatalogScreen,
        parent: Option<Box<ActiveScreen>>,
    },
    ExtensionSettings {
        owner: String,
        parent: Option<Box<ActiveScreen>>,
    },
}
impl ActiveScreen {
    fn migrated_screen_mut(&mut self) -> Option<&mut dyn Screen> {
        match self {
            Self::Title(screen) => Some(screen),
            Self::Pause(screen) => Some(screen),
            Self::Options(screen) => Some(screen),
            Self::Video(screen) => Some(screen),
            Self::SkinCustomization(screen) => Some(screen),
            Self::ChatOptions(screen) => Some(screen),
            Self::SoundOptions(screen) => Some(screen),
            Self::OnlineOptions(screen) => Some(screen),
            Self::TelemetryInfo(screen) => Some(screen),
            Self::ServerLinks(screen) => Some(screen),
            Self::ServerDialog { screen, .. } => Some(screen),
            Self::WorldOptions(screen) => Some(screen),
            Self::GameRules(screen) => Some(screen),
            Self::Controls(screen) => Some(screen),
            Self::CreditsAndAttribution(screen) => Some(screen),
            Self::WinScreen { screen, .. } => Some(screen),
            Self::DeathScreen(screen) => Some(screen),
            Self::Stats(screen) => Some(screen),
            Self::Advancements(screen) => Some(screen),
            Self::AccessibilityOptions(screen) => Some(screen),
            Self::MouseSettings(screen) => Some(screen),
            Self::Restrictions(screen) => Some(screen),
            Self::LanguageSelect(screen) => Some(screen),
            Self::KeyBinds { screen, .. } => Some(screen),
            Self::InBedChat(screen) => Some(screen),
            Self::Multiplayer(screen) => Some(screen),
            Self::ManageServer(screen) => Some(screen),
            Self::DirectConnect(screen) => Some(screen),
            Self::Connecting(screen) => Some(screen),
            Self::Disconnected(screen) => Some(screen),
            Self::Safety(screen) => Some(screen),
            Self::CodeOfConduct(screen) => Some(screen),
            Self::Confirm(screen) => Some(screen),
            Self::ConfirmLink(screen) => Some(screen),
            Self::FriendsConfirm { screen, .. } => Some(screen),
            Self::PackConfirm(screen) => Some(screen),
            Self::ServerReconfig(screen) => Some(screen),
            Self::ResourcePacks(screen) => Some(screen),
            Self::NavigationTransition
            | Self::Container(_)
            | Self::SubmenuStub { .. }
            | Self::Chat { .. }
            | Self::LevelLoading(_)
            | Self::FontSettings(_)
            | Self::ExtensionCatalog { .. }
            | Self::ExtensionSettings { .. } => None,
        }
    }
    fn migrated_screen_ref(&self) -> Option<&dyn Screen> {
        match self {
            Self::Title(screen) => Some(screen),
            Self::Pause(screen) => Some(screen),
            Self::Options(screen) => Some(screen),
            Self::Video(screen) => Some(screen),
            Self::SkinCustomization(screen) => Some(screen),
            Self::ChatOptions(screen) => Some(screen),
            Self::SoundOptions(screen) => Some(screen),
            Self::OnlineOptions(screen) => Some(screen),
            Self::TelemetryInfo(screen) => Some(screen),
            Self::ServerLinks(screen) => Some(screen),
            Self::ServerDialog { screen, .. } => Some(screen),
            Self::WorldOptions(screen) => Some(screen),
            Self::GameRules(screen) => Some(screen),
            Self::Controls(screen) => Some(screen),
            Self::CreditsAndAttribution(screen) => Some(screen),
            Self::WinScreen { screen, .. } => Some(screen),
            Self::DeathScreen(screen) => Some(screen),
            Self::Stats(screen) => Some(screen),
            Self::Advancements(screen) => Some(screen),
            Self::AccessibilityOptions(screen) => Some(screen),
            Self::MouseSettings(screen) => Some(screen),
            Self::Restrictions(screen) => Some(screen),
            Self::LanguageSelect(screen) => Some(screen),
            Self::KeyBinds { screen, .. } => Some(screen),
            Self::InBedChat(screen) => Some(screen),
            Self::Multiplayer(screen) => Some(screen),
            Self::ManageServer(screen) => Some(screen),
            Self::DirectConnect(screen) => Some(screen),
            Self::Connecting(screen) => Some(screen),
            Self::Disconnected(screen) => Some(screen),
            Self::Safety(screen) => Some(screen),
            Self::CodeOfConduct(screen) => Some(screen),
            Self::Confirm(screen) => Some(screen),
            Self::ConfirmLink(screen) => Some(screen),
            Self::FriendsConfirm { screen, .. } => Some(screen),
            Self::PackConfirm(screen) => Some(screen),
            Self::ServerReconfig(screen) => Some(screen),
            Self::ResourcePacks(screen) => Some(screen),
            Self::NavigationTransition
            | Self::Container(_)
            | Self::SubmenuStub { .. }
            | Self::Chat { .. }
            | Self::LevelLoading(_)
            | Self::FontSettings(_)
            | Self::ExtensionCatalog { .. }
            | Self::ExtensionSettings { .. } => None,
        }
    }
    fn screen_instance_id(&self) -> Option<u64> {
        self.migrated_screen_ref()
            .map(|screen| screen.base_ref().instance_id())
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ServerConnectionOrigin {
    #[default]
    Normal,
    QuickPlayMultiplayer,
}
impl ServerConnectionOrigin {
    pub const fn is_quick_play(self) -> bool {
        matches!(self, Self::QuickPlayMultiplayer)
    }
}
#[derive(Clone, Debug, Default)]
struct InProcessClipboard {
    contents: String,
}
impl el_gui::components::edit_box::Clipboard for InProcessClipboard {
    fn get(&self) -> String {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            if let Ok(mut board) = arboard::Clipboard::new() {
                if let Ok(text) = board.get_text() {
                    return text;
                }
            }
        }
        self.contents.clone()
    }
    fn set(&mut self, value: String) {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            if let Ok(mut board) = arboard::Clipboard::new() {
                let _ = board.set_text(value.clone());
            }
        }
        self.contents = value;
    }
}
fn menu_background_blur_radius_for(screen: Option<&ActiveScreen>, blur_radius: f32) -> f32 {
    if blur_radius < 1.0 {
        return 0.0;
    }
    match screen {
        None
        | Some(ActiveScreen::Title(_))
        | Some(ActiveScreen::Chat { .. })
        | Some(ActiveScreen::InBedChat(_))
        | Some(ActiveScreen::NavigationTransition)
        | Some(ActiveScreen::Container(_))
        | Some(ActiveScreen::DeathScreen(_)) => 0.0,
        Some(ActiveScreen::Confirm(screen)) if screen.uses_death_screen_background() => 0.0,
        Some(_) => blur_radius,
    }
}
fn screen_scene_flags(screen: Option<&ActiveScreen>) -> (bool, bool) {
    let Some(screen) = screen else {
        return (true, false);
    };
    let is_title = matches!(screen, ActiveScreen::Title(_));
    let in_world = match screen {
        ActiveScreen::Disconnected(_) | ActiveScreen::Connecting(_) => false,
        _ => screen.is_in_world(),
    };
    (in_world, is_title)
}
struct ClientAdvancementIconRenderer;
impl AdvancementIconRenderer for ClientAdvancementIconRenderer {
    fn extract_icon(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        display: &AdvancementDisplayData,
        x: i32,
        y: i32,
        _size: i32,
    ) {
        let Some(location) = ResourceLocation::try_from_string(&display.icon_registry_id) else {
            return;
        };
        let item = el_core::registry::BuiltInRegistries::items().get_by_location(&location);
        if item.location != location {
            return;
        }
        let stack = el_core::item::ItemStack::new(item.clone(), display.icon_count);
        graphics.render_item(stack, x, y);
    }
}
fn advancement_icon_stack(display: &AdvancementDisplayData) -> Option<el_core::item::ItemStack> {
    let location = ResourceLocation::try_from_string(&display.icon_registry_id)?;
    let item = el_core::registry::BuiltInRegistries::items().get_by_location(&location);
    if item.location != location {
        return None;
    }
    Some(el_core::item::ItemStack::new(
        item.clone(),
        display.icon_count,
    ))
}
const MENU_BACKGROUND_ALPHA: f32 = 64.0 / 255.0;
const MENU_LIST_BACKGROUND_ALPHA: f32 = 112.0 / 255.0;
fn composite_alpha_over(base: f32, overlay: f32) -> f32 {
    1.0 - (1.0 - base) * (1.0 - overlay)
}
fn is_valid_server_favicon(bytes: &[u8]) -> bool {
    image::load_from_memory(bytes).ok().is_some_and(|image| {
        image.width() <= 1024
            && image.height() <= 1024
            && image.width() == 64
            && image.height() == 64
    })
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum GuiInputType {
    #[default]
    None,
    Mouse,
    KeyboardArrow,
    KeyboardTab,
}
impl GuiInputType {
    fn is_keyboard(self) -> bool {
        matches!(self, Self::KeyboardArrow | Self::KeyboardTab)
    }
}
fn gui_mouse_button_id(button: MouseButton) -> u16 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
        MouseButton::Back => 3,
        MouseButton::Forward => 4,
        MouseButton::Other(button) => button,
    }
}
fn is_java_double_click(
    last_click: Option<&(web_time::Instant, u64, u16)>,
    now: web_time::Instant,
    screen_id: u64,
    button_id: u16,
) -> bool {
    last_click.is_some_and(|(last_time, last_screen_id, last_button_id)| {
        *last_screen_id == screen_id
            && *last_button_id == button_id
            && now.duration_since(*last_time) < std::time::Duration::from_millis(250)
    })
}
#[derive(Clone, Debug)]
pub struct LevelLoadingScreen {
    preserved_chat: Option<PreservedLevelLoadingChat>,
}
#[derive(Clone, Debug)]
struct PreservedLevelLoadingChat {
    input: String,
    is_draft: bool,
}
#[derive(Clone, Debug)]
pub enum ServerResourcePackResponse {
    Accept {
        id: uuid::Uuid,
        url: String,
        hash: String,
        required: bool,
    },
    Decline {
        id: uuid::Uuid,
        required: bool,
    },
}
impl ActiveScreen {
    pub fn is_in_world(&self) -> bool {
        match self {
            Self::Title(_) => false,
            Self::NavigationTransition => false,
            Self::Pause(_) => true,
            Self::Chat { .. } | Self::InBedChat(_) => true,
            Self::Container(_) => true,
            Self::Options(s) => s.in_world,
            Self::Video(s) => s.in_world,
            Self::SkinCustomization(s) => s.in_world,
            Self::ChatOptions(s) => s.in_world,
            Self::SoundOptions(s) => s.in_world,
            Self::OnlineOptions(screen) => screen.in_world(),
            Self::TelemetryInfo(screen) => screen.in_world(),
            Self::ServerLinks(screen) => screen.in_world(),
            Self::ServerDialog { in_world, .. } => *in_world,
            Self::WorldOptions(s) => s.in_world,
            Self::GameRules(s) => s.in_world(),
            Self::Controls(s) => s.in_world,
            Self::CreditsAndAttribution(s) => s.in_world,
            Self::WinScreen { screen, .. } => screen.poem(),
            Self::DeathScreen(_) => true,
            Self::Stats(_) => true,
            Self::Advancements(_) => true,
            Self::AccessibilityOptions(s) => s.in_world,
            Self::MouseSettings(s) => s.in_world,
            Self::Restrictions(s) => s.in_world,
            Self::LanguageSelect(s) => s.in_world,
            Self::FontSettings(s) => s.in_world,
            Self::KeyBinds { screen, .. } => screen.in_world,
            Self::SubmenuStub { in_world, .. } => *in_world,
            Self::Multiplayer(_) => false,
            Self::ManageServer(s) => s.in_world,
            Self::DirectConnect(s) => s.in_world,
            Self::Connecting(s) => s.in_world(),
            Self::LevelLoading(_) => true,
            Self::Disconnected(s) => s.in_world,
            Self::Safety(s) => s.in_world,
            Self::CodeOfConduct(s) => s.in_world,
            Self::Confirm(s) => s.in_world,
            Self::ConfirmLink(s) => s.in_world,
            Self::FriendsConfirm { in_world, .. } => *in_world,
            Self::PackConfirm(s) => s.in_world,
            Self::ServerReconfig(s) => s.in_world(),
            Self::ResourcePacks(s) => s.in_world,
            Self::ExtensionCatalog { screen, .. } => screen.in_world,
            Self::ExtensionSettings { parent, .. } => {
                parent.as_ref().is_some_and(|parent| parent.is_in_world())
            }
        }
    }
    fn blocks_hud(&self) -> bool {
        match self {
            Self::LevelLoading(_) => true,
            _ => !self.is_in_world(),
        }
    }
}
struct GuiPrepareProfiler<'a> {
    profiler: Option<&'a mut RenderProfiler>,
}
impl<'a> GuiPrepareProfiler<'a> {
    fn new(profiler: Option<&'a mut RenderProfiler>) -> Self {
        Self { profiler }
    }
    fn started(&self) -> Option<web_time::Instant> {
        self.profiler.as_ref().map(|_| web_time::Instant::now())
    }
    fn observe(&mut self, name: &'static str, started_at: Option<web_time::Instant>) {
        if let (Some(profiler), Some(started_at)) = (self.profiler.as_deref_mut(), started_at) {
            profiler.observe(name, started_at.elapsed());
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScreenCacheKey {
    pub screen_tag: u32,
    pub logical_w: u32,
    pub logical_h: u32,
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub anim_phase: u64,
    pub fov_bits: u32,
    pub hide_server_address: bool,
    pub defer_inventory_tooltip: bool,
    pub custom_textures_len: usize,
    pub extra_hash: u64,
}
#[derive(Default)]
pub struct ScreenRenderCache {
    pub state_key: Option<ScreenCacheKey>,
    pub vertices: Vec<GuiVertex>,
    pub indices: Vec<u32>,
    pub batches: Vec<DrawBatch>,
    pub pending_cursor: el_gui::input::events::CursorType,
    pub pending_chat_tooltip: Option<String>,
}
fn screen_cursor_over_live_chat(
    live_chat_cursor: Option<el_gui::input::events::CursorType>,
    screen_cursor: el_gui::input::events::CursorType,
) -> el_gui::input::events::CursorType {
    if screen_cursor == el_gui::input::events::CursorType::Default {
        live_chat_cursor.unwrap_or(screen_cursor)
    } else {
        screen_cursor
    }
}
fn chat_hover_item_stack(
    id: &str,
    count: u32,
    tag: Option<&[u8]>,
) -> Option<el_core::item::ItemStack> {
    let mut fields = vec![
        ("id".into(), el_nbt::Tag::String(id.to_string())),
        ("count".into(), el_nbt::Tag::Int(i32::try_from(count).ok()?)),
    ];
    if let Some(bytes) = tag {
        if bytes.len() > 1_048_576 {
            return None;
        }
        fields.push((
            "components".into(),
            el_nbt::io::read_any_compound(bytes).ok()?,
        ));
    }
    el_core::item::ItemStack::from_nbt(&el_nbt::Tag::Compound(fields))
        .filter(|item| !item.is_empty())
}
fn pause_dialog_addition(
    dialog: Option<&(String, Option<serde_json::Value>, bool)>,
    has_links: bool,
) -> (bool, Option<String>) {
    let Some((_, data, requires_links)) = dialog else {
        return (has_links, None);
    };
    let title = data
        .as_ref()
        .and_then(|data| data.get("external_title").or_else(|| data.get("title")))
        .map(crate::ui::gui::screen::container::styled_text_component);
    (
        (!requires_links || has_links) && (data.is_some() || *requires_links),
        title,
    )
}
fn server_pack_status_after_decision(accepted: bool) -> u8 {
    if accepted {
        el_protocol_backend::SERVER_PACK_STATUS_ENABLED
    } else {
        el_protocol_backend::SERVER_PACK_STATUS_DISABLED
    }
}
fn install_pack_prompt(
    previous_screen: Option<ActiveScreen>,
    pending_parent: &mut Option<Box<ActiveScreen>>,
    mut screen: PackConfirmScreen,
) -> ActiveScreen {
    match previous_screen {
        Some(ActiveScreen::PackConfirm(previous)) => {
            let mut requests = previous.requests;
            requests.append(&mut screen.requests);
            screen.requests = requests;
        }
        parent => *pending_parent = parent.map(Box::new),
    }
    ActiveScreen::PackConfirm(screen)
}
pub struct GuiManager {
    pub renderer: GuiRenderer,
    pub font: std::sync::Arc<FontRenderer>,
    pub hud: HudRenderer,
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    touch_controls: el_launcher_core::touch_controls::TouchControlProfile,
    #[cfg(target_os = "android")]
    touch_visual: crate::input::android_touch::TouchVisualState,
    #[cfg(target_os = "android")]
    touch_safe_insets: [f32; 4],
    active_screen: Option<ActiveScreen>,
    menu_parent_stack: Vec<ActiveScreen>,
    pending_pack_parent: Option<Box<ActiveScreen>>,
    live_server_pack_status: Option<u8>,
    extension_catalog: ExtensionCatalog,
    pending_extension_settings_request: Option<String>,
    code_of_conduct_parent: Option<ActiveScreen>,
    pending_window_fullscreen: Option<bool>,
    pending_window_video_mode_change: bool,
    fullscreen_video_mode_dirty: bool,
    fullscreen_video_modes: Vec<FullscreenVideoMode>,
    pending_font_reload: bool,
    resource_manager: ClientResourceManager,
    resource_reload_pending: bool,
    has_singleplayer_server: bool,
    is_local_server: bool,
    clipboard: Rc<RefCell<InProcessClipboard>>,
    pending_sound_category_changes: Vec<crate::client::sound_engine::SoundSource>,
    audio_output_catalog: Option<crate::client::audio::AudioOutputCatalog>,
    now_playing_title: Option<String>,
    now_playing_started_at: Option<web_time::Instant>,
    pending_audio_output_refresh: bool,
    pending_audio_output_selection: Option<(u64, crate::client::audio::AudioOutputSelection)>,
    pending_directional_audio: Option<bool>,
    pending_ui_sounds: Vec<el_gui::components::WidgetClickSound>,
    terrain_mesh_rebuild_pending: bool,
    pending_win_screen_respawn_request: bool,
    pending_death_screen_respawn_request: bool,
    pending_statistics_request: bool,
    pending_game_rule_values_request: bool,
    pending_game_rule_edits: Vec<(String, String)>,
    pending_sign_update: Option<([i32; 3], bool, [String; 4])>,
    statistics_values: std::collections::HashMap<menu_data::StatKey, i32>,
    advancements_snapshot: AdvancementsSnapshot,
    selected_advancements_tab: Option<String>,
    pending_seen_advancement_actions: Vec<SeenAdvancementsAction>,
    server_links: Vec<(String, String)>,
    pause_screen_dialog: Option<(String, Option<serde_json::Value>, bool)>,
    pub hud_hidden: bool,
    pub mouse_x: f32,
    pub mouse_y: f32,
    pub physical_mouse_x: f32,
    pub physical_mouse_y: f32,
    pub gui_scale_setting: u32,
    pub fov_setting: f32,
    pub sensitivity_setting: f64,
    pub mouse_wheel_sensitivity: f32,
    pub invert_mouse_x: bool,
    pub invert_mouse_y: bool,
    pub view_bobbing: bool,
    pub damage_tilt_strength: f64,
    pub toggle_crouch: bool,
    pub toggle_sprint: bool,
    pub toggle_attack: bool,
    pub toggle_use: bool,
    pub smooth_camera: bool,
    pub discrete_mouse_scroll: bool,
    pub allow_cursor_changes: bool,
    pub pending_cursor: el_gui::input::events::CursorType,
    pub last_screen_w: f32,
    pub last_screen_h: f32,
    pub options_path: std::path::PathBuf,
    resource_pack_directory: Option<std::path::PathBuf>,
    command_history_path: std::path::PathBuf,
    pub recent_chat: Vec<String>,
    pub chat_component: crate::ui::chat::ChatComponent,
    realms_texture_manager: RealmsTextureManager,
    gui_ticks: u32,
    panorama_spin: f32,
    last_realtime_delta_at: web_time::Instant,
    chat_cursor: usize,
    chat_input_display_pos: usize,
    chat_selection_anchor: Option<usize>,
    last_chat_click_time: Option<web_time::Instant>,
    last_chat_click_cursor: usize,
    chat_clipboard: String,
    chat_history_pos: usize,
    chat_history_buffer: String,
    chat_suggestion_index: usize,
    chat_suggestion_offset: usize,
    chat_tab_cycles: bool,
    chat_suggestion_original_contents: Option<String>,
    active_chat_suggestions: Vec<crate::ui::chat::ChatSuggestion>,
    chat_suggestions_allowed: bool,
    chat_suggestions_popup_open: bool,
    chat_is_draft: bool,
    pending_link_return_input: Option<String>,
    control_held: bool,
    alt_held: bool,
    mouse_down_left: bool,
    last_gui_input_type: GuiInputType,
    last_migrated_screen_click: Option<(web_time::Instant, u64, u16)>,
    connected_chat: bool,
    pending_chat_submissions: Vec<String>,
    pending_unattended_chat_commands: Vec<String>,
    pending_custom_click_actions: Vec<(String, Option<Vec<u8>>)>,
    pub pending_server_resource_pack_responses: Vec<ServerResourcePackResponse>,
    server_command_names: Vec<String>,
    server_command_root: u32,
    server_command_nodes: Vec<crate::ui::domain::CommandNode>,
    online_player_names: Vec<String>,
    pub tab_list_header: String,
    pub tab_list_footer: String,
    pub scoreboard: crate::ui::scoreboard::Scoreboard,
    server_title: Option<String>,
    server_subtitle: Option<String>,
    title_time: u32,
    title_fade_in_time: u32,
    title_stay_time: u32,
    title_fade_out_time: u32,
    player_infos: std::collections::HashMap<uuid::Uuid, crate::ui::scoreboard::PlayerInfoState>,
    tablist_level_players: std::collections::HashSet<uuid::Uuid>,
    tab_health_states: std::collections::HashMap<uuid::Uuid, scoreboard_hud::TabHealthState>,
    show_player_list: bool,
    online_mode: bool,
    custom_chat_completions: Vec<String>,
    server_chat_suggestions: Vec<crate::ui::chat::ChatSuggestion>,
    server_suggestion_input: Option<String>,
    latest_suggestion_request_id: u32,
    last_suggestion_request_input: Option<String>,
    pending_chat_suggestion_requests: Vec<(u32, String)>,
    requested_game_mode: Option<GameType>,
    requested_difficulty: Option<crate::world::client_level::Difficulty>,
    remote_world_difficulty: Option<(crate::world::client_level::Difficulty, bool)>,
    requested_time: Option<TimeRequest>,
    requested_weather: Option<WeatherMode>,
    requested_disconnect: bool,
    requested_quit: bool,
    pub extra_options: std::collections::HashMap<String, String>,
    pub pending_container_clicks: Vec<ServerboundContainerClickPacket>,
    pub pending_container_slot_state_changes: Vec<(u32, i32, bool)>,
    pub pending_creative_clicks: Vec<ServerboundSetCreativeModeSlotPacket>,
    pub pending_bundle_item_selections: Vec<(i32, u32)>,
    pending_chat_tooltip: Option<String>,
    pending_chat_hover: Option<crate::ui::domain::ChatHoverAction>,
    autoshot_logical_mouse: Option<(f32, f32)>,
    bundle_item_selection_connection_available: bool,
    pub requested_container_close: Option<i32>,
    pub shift_held: bool,
    held_modifier_sides: u8,
    title_version_name: String,
    title_demo: bool,
    title_multiplayer_allowed: bool,
    cached_title_splash: Option<Option<String>>,
    player_username: String,
    player_profile_id: Option<uuid::Uuid>,
    protocol_backend: ProtocolBackendData,
    game_rule_editor_supported: bool,
    server_connection: Option<RemoteConnectionHandle>,
    server_connection_is_transfer: bool,
    server_connection_origin: ServerConnectionOrigin,
    pub rendering_in_world: bool,
    pub rendering_title_screen: bool,
    pub panorama_already_rendered: bool,
    pub ping_results_rx:
        Option<crossbeam_channel::Receiver<(usize, el_gui::screens::multiplayer::ServerEntry)>>,
    #[cfg(not(target_arch = "wasm32"))]
    ping_results_tx:
        Option<crossbeam_channel::Sender<(usize, el_gui::screens::multiplayer::ServerEntry)>>,
    lan_servers_rx: Option<crossbeam_channel::Receiver<Vec<LanServerEntry>>>,
    lan_detector_stop: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    stashed_multiplayer_screen: Option<MultiplayerScreen>,
    connecting_server_ip: Option<String>,
    pending_resource_packs: Option<ResourcePacksScreen>,
    pub screen_dirty: bool,
    screen_cache: ScreenRenderCache,
}
fn gui_text_obfuscation_seed(text: &GuiTextRenderState, index: usize) -> u64 {
    let mut seed = 0xcbf2_9ce4_8422_2325u64;
    for byte in text.text.bytes() {
        seed = (seed ^ u64::from(byte)).wrapping_mul(0x100_0000_01b3);
    }
    for part in [
        text.x as u64,
        text.y as u64,
        u64::from(text.pose.m20.to_bits()),
        u64::from(text.pose.m21.to_bits()),
        index as u64,
    ] {
        seed = (seed ^ part).wrapping_mul(0x100_0000_01b3);
    }
    seed
}
fn apply_player_info_update_entries(
    player_infos: &mut std::collections::HashMap<
        uuid::Uuid,
        crate::ui::scoreboard::PlayerInfoState,
    >,
    entries: Vec<PlayerInfoUpdateEntry>,
) {
    for entry in entries {
        let is_add = entry.add_player.is_some();
        let info = if is_add {
            player_infos.entry(entry.uuid).or_insert_with(|| {
                crate::ui::scoreboard::PlayerInfoState {
                    uuid: entry.uuid,
                    name: String::new(),
                    texture_url: None,
                    model: None,
                    game_mode: GameType::Survival,
                    latency: 0,
                    display_name: None,
                    listed: false,
                    list_order: 0,
                    show_hat: true,
                }
            })
        } else if let Some(info) = player_infos.get_mut(&entry.uuid) {
            info
        } else {
            continue;
        };
        if let Some(add) = entry.add_player {
            info.name = add.name;
            info.texture_url = add.texture_url;
            info.model = add.model;
        }
        if let Some(game_mode) = entry.game_mode {
            info.game_mode = GameType::by_id(game_mode as i32);
        }
        if let Some(listed) = entry.listed {
            info.listed = listed;
        }
        if let Some(latency) = entry.latency {
            info.latency = latency;
        }
        if let Some(update) = entry.display_name {
            use crate::ui::domain::PlayerInfoDisplayNameUpdate;
            info.display_name = match update {
                PlayerInfoDisplayNameUpdate::Clear => None,
                PlayerInfoDisplayNameUpdate::Set { text } => Some(text),
            };
        }
        if let Some(list_order) = entry.list_order {
            info.list_order = list_order;
        }
        if let Some(show_hat) = entry.show_hat {
            info.show_hat = show_hat;
        }
    }
}
impl GuiManager {
    pub(crate) fn set_portal_game_time(&mut self, game_time: f32) {
        self.renderer.portal_game_time = game_time;
    }
    #[cfg(target_os = "android")]
    pub fn set_android_controls_profile(
        &mut self,
        profile: el_launcher_core::touch_controls::TouchControlProfile,
    ) {
        self.touch_controls = profile;
        self.touch_visual = Default::default();
    }
    #[cfg(target_os = "android")]
    pub fn android_controls_profile(
        &self,
    ) -> &el_launcher_core::touch_controls::TouchControlProfile {
        &self.touch_controls
    }
    #[cfg(target_os = "android")]
    pub fn set_android_touch_visual(
        &mut self,
        visual: crate::input::android_touch::TouchVisualState,
    ) {
        self.touch_visual = visual;
    }
    #[cfg(target_os = "android")]
    pub fn set_android_touch_insets(&mut self, insets: [f32; 4]) {
        self.touch_safe_insets = insets;
    }
    pub fn new(
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
        target_format: wgpu::TextureFormat,
    ) -> Result<Self, ResourceError> {
        let renderer = GuiRenderer::new(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            target_format,
        )?;
        let font = std::sync::Arc::new(FontRenderer::load(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            FontOptions::default(),
        )?);
        let mut hud = HudRenderer::new();
        hud.reload_locator_waypoint_styles(resource_manager);
        let options_path = resource_manager
            .assets_root()
            .parent()
            .and_then(|p| p.parent())
            .unwrap_or(&std::path::Path::new("."))
            .join("options.txt");
        let command_history_path = options_path.with_file_name("command_history.txt");
        let mut manager = Self {
            renderer,
            font,
            hud,
            #[cfg(target_os = "android")]
            touch_controls: el_launcher_core::touch_controls::TouchControlProfile::default(),
            #[cfg(not(target_os = "android"))]
            touch_controls: el_launcher_core::touch_controls::TouchControlProfile::for_preset(
                el_launcher_core::touch_controls::TouchPreset::Disabled,
            ),
            #[cfg(target_os = "android")]
            touch_visual: Default::default(),
            #[cfg(target_os = "android")]
            touch_safe_insets: [0.0; 4],
            active_screen: None,
            menu_parent_stack: Vec::new(),
            pending_pack_parent: None,
            live_server_pack_status: None,
            extension_catalog: ExtensionCatalog::default(),
            pending_extension_settings_request: None,
            code_of_conduct_parent: None,
            pending_window_fullscreen: None,
            pending_window_video_mode_change: false,
            fullscreen_video_mode_dirty: false,
            fullscreen_video_modes: Vec::new(),
            pending_font_reload: false,
            resource_manager: resource_manager.clone(),
            resource_reload_pending: false,
            has_singleplayer_server: false,
            is_local_server: false,
            clipboard: Rc::new(RefCell::new(InProcessClipboard::default())),
            pending_sound_category_changes: Vec::new(),
            audio_output_catalog: None,
            now_playing_title: None,
            now_playing_started_at: None,
            pending_audio_output_refresh: false,
            pending_audio_output_selection: None,
            pending_directional_audio: None,
            pending_ui_sounds: Vec::new(),
            terrain_mesh_rebuild_pending: false,
            pending_win_screen_respawn_request: false,
            pending_death_screen_respawn_request: false,
            pending_statistics_request: false,
            pending_game_rule_values_request: false,
            pending_game_rule_edits: Vec::new(),
            pending_sign_update: None,
            statistics_values: std::collections::HashMap::new(),
            advancements_snapshot: AdvancementsSnapshot::default(),
            selected_advancements_tab: None,
            pending_seen_advancement_actions: Vec::new(),
            server_links: Vec::new(),
            pause_screen_dialog: None,
            hud_hidden: false,
            mouse_x: 0.0,
            mouse_y: 0.0,
            physical_mouse_x: 0.0,
            physical_mouse_y: 0.0,
            gui_scale_setting: 0,
            fov_setting: 0.5,
            sensitivity_setting: 0.5,
            mouse_wheel_sensitivity: 1.0,
            invert_mouse_x: false,
            invert_mouse_y: false,
            view_bobbing: true,
            damage_tilt_strength: 1.0,
            toggle_crouch: false,
            toggle_sprint: false,
            toggle_attack: false,
            toggle_use: false,
            smooth_camera: false,
            discrete_mouse_scroll: false,
            allow_cursor_changes: true,
            pending_cursor: el_gui::input::events::CursorType::Default,
            last_screen_w: 1280.0,
            last_screen_h: 720.0,
            options_path,
            resource_pack_directory: None,
            command_history_path,
            recent_chat: Vec::new(),
            chat_component: crate::ui::chat::ChatComponent::new(),
            realms_texture_manager: RealmsTextureManager::default(),
            gui_ticks: 0,
            panorama_spin: 0.0,
            last_realtime_delta_at: web_time::Instant::now(),
            chat_cursor: 0,
            chat_input_display_pos: 0,
            chat_selection_anchor: None,
            last_chat_click_time: None,
            last_chat_click_cursor: 0,
            chat_clipboard: String::new(),
            chat_history_pos: 0,
            chat_history_buffer: String::new(),
            chat_suggestion_index: 0,
            chat_suggestion_offset: 0,
            chat_tab_cycles: false,
            chat_suggestion_original_contents: None,
            active_chat_suggestions: Vec::new(),
            chat_suggestions_allowed: false,
            chat_suggestions_popup_open: false,
            chat_is_draft: false,
            pending_link_return_input: None,
            control_held: false,
            alt_held: false,
            last_gui_input_type: GuiInputType::None,
            last_migrated_screen_click: None,
            connected_chat: false,
            pending_chat_submissions: Vec::new(),
            pending_unattended_chat_commands: Vec::new(),
            pending_custom_click_actions: Vec::new(),
            pending_server_resource_pack_responses: Vec::new(),
            server_command_names: Vec::new(),
            server_command_root: 0,
            server_command_nodes: Vec::new(),
            online_player_names: Vec::new(),
            tab_list_header: String::new(),
            tab_list_footer: String::new(),
            scoreboard: crate::ui::scoreboard::Scoreboard::default(),
            server_title: None,
            server_subtitle: None,
            title_time: 0,
            title_fade_in_time: 10,
            title_stay_time: 70,
            title_fade_out_time: 20,
            player_infos: std::collections::HashMap::new(),
            tablist_level_players: std::collections::HashSet::new(),
            tab_health_states: std::collections::HashMap::new(),
            show_player_list: false,
            online_mode: false,
            custom_chat_completions: Vec::new(),
            server_chat_suggestions: Vec::new(),
            server_suggestion_input: None,
            latest_suggestion_request_id: 0,
            last_suggestion_request_input: None,
            pending_chat_suggestion_requests: Vec::new(),
            requested_game_mode: None,
            requested_difficulty: None,
            remote_world_difficulty: None,
            requested_time: None,
            requested_weather: None,
            requested_disconnect: false,
            requested_quit: false,
            extra_options: std::collections::HashMap::new(),
            pending_container_clicks: Vec::new(),
            pending_container_slot_state_changes: Vec::new(),
            pending_creative_clicks: Vec::new(),
            pending_bundle_item_selections: Vec::new(),
            pending_chat_tooltip: None,
            pending_chat_hover: None,
            autoshot_logical_mouse: crate::ui::gui::autoshot::AutoShot::from_env()
                .and_then(|shot| shot.mouse),
            bundle_item_selection_connection_available: false,
            requested_container_close: None,
            shift_held: false,
            held_modifier_sides: 0,
            mouse_down_left: false,
            title_version_name: el_launcher_core::ELTHARN_CLIENT_ASSETS_COMPATIBILITY.to_string(),
            title_demo: false,
            title_multiplayer_allowed: true,
            cached_title_splash: None,
            player_username: String::new(),
            player_profile_id: None,
            protocol_backend: ProtocolBackendData {
                requested_version: el_protocol_backend::VANILLA_VERSION_NAME.to_string(),
                backend_path: None,
            },
            game_rule_editor_supported: false,
            server_connection: None,
            server_connection_is_transfer: false,
            server_connection_origin: ServerConnectionOrigin::Normal,
            rendering_in_world: true,
            rendering_title_screen: false,
            panorama_already_rendered: false,
            ping_results_rx: None,
            #[cfg(not(target_arch = "wasm32"))]
            ping_results_tx: None,
            lan_servers_rx: None,
            lan_detector_stop: None,
            stashed_multiplayer_screen: None,
            connecting_server_ip: None,
            pending_resource_packs: None,
            screen_dirty: true,
            screen_cache: ScreenRenderCache::default(),
        };
        manager.load_options();
        let font_options = manager.font_options();
        if font_options != FontOptions::default() {
            manager.reload_font(device, queue, texture_bind_group_layout)?;
        }
        manager.load_command_history();
        el_gui::register_translator(|key| crate::ui::i18n::translate(key).to_string());
        Ok(manager)
    }
    pub fn mark_screen_dirty(&mut self) {
        self.screen_dirty = true;
        self.screen_cache.state_key = None;
    }
    pub fn resource_manager_clone(&self) -> ClientResourceManager {
        self.resource_manager.clone()
    }
    pub fn take_resource_reload_request(&mut self) -> bool {
        std::mem::take(&mut self.resource_reload_pending)
    }
    pub fn take_window_fullscreen_request(&mut self) -> Option<bool> {
        self.pending_window_fullscreen.take()
    }
    pub fn set_fullscreen_video_modes(&mut self, modes: Vec<FullscreenVideoMode>) {
        self.fullscreen_video_modes = modes;
    }
    pub fn take_window_video_mode_change_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_window_video_mode_change)
    }
    fn commit_fullscreen_video_mode_change(&mut self) {
        if std::mem::take(&mut self.fullscreen_video_mode_dirty) {
            self.pending_window_video_mode_change = true;
        }
    }
    pub fn take_font_reload_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_font_reload)
    }
    pub fn reload_font(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Result<(), ResourceError> {
        let font = FontRenderer::load(
            device,
            queue,
            texture_bind_group_layout,
            &self.resource_manager,
            self.font_options(),
        )?;
        self.font = std::sync::Arc::new(font);
        Ok(())
    }
    fn font_options(&self) -> FontOptions {
        FontOptions {
            uniform: self
                .extra_options
                .get("forceUnicodeFont")
                .is_some_and(|value| value == "true"),
            japanese_variants: self
                .extra_options
                .get("japaneseGlyphVariants")
                .is_some_and(|value| value == "true"),
        }
    }
    fn set_font_option(&mut self, key: &str, enabled: bool) {
        let value = if enabled { "true" } else { "false" };
        if self.extra_options.get(key).map(String::as_str) == Some(value) {
            return;
        }
        self.extra_options
            .insert(key.to_string(), value.to_string());
        self.pending_font_reload = true;
        self.save_options();
        if key == "forceUnicodeFont" {
            self.update_logical_mouse();
        }
        self.mark_screen_dirty();
    }
    pub fn sync_fullscreen_option_from_window(&mut self, enabled: bool) {
        let value = if enabled { "true" } else { "false" };
        if self.extra_options.get("fullscreen").map(String::as_str) != Some(value) {
            self.extra_options
                .insert("fullscreen".to_string(), value.to_string());
            self.save_options();
        }
    }
    pub fn take_pending_sound_category_changes(
        &mut self,
    ) -> Vec<crate::client::sound_engine::SoundSource> {
        std::mem::take(&mut self.pending_sound_category_changes)
    }
    pub fn audio_output_preference(&self) -> String {
        self.extra_options
            .get("soundDevice")
            .cloned()
            .unwrap_or_default()
    }
    pub fn music_started(&mut self, sound_file: &ResourceLocation) {
        let Some(key) = crate::client::music_manager::music_sound_language_key(sound_file) else {
            self.clear_current_music();
            return;
        };
        let translated = crate::ui::i18n::translate(&key);
        self.now_playing_title = Some(if translated == key {
            key.rsplit('.').next().unwrap_or(&key).replace('_', " ")
        } else {
            translated.into_owned()
        });
        self.now_playing_started_at = Some(web_time::Instant::now());
    }
    pub fn clear_current_music(&mut self) {
        self.now_playing_title = None;
        self.now_playing_started_at = None;
    }
    pub fn refresh_now_playing_toast_for_volume_change(
        &mut self,
        source: crate::client::sound_engine::SoundSource,
    ) {
        let display_mode = self.extra_options.get("musicToast").map(String::as_str);
        let music_volume =
            crate::client::sound_engine::SoundSourceVolumes::from_options(&self.extra_options)
                .final_source_volume(crate::client::sound_engine::SoundSource::Music);
        if display_mode.is_some_and(|mode| mode != "never")
            && self.now_playing_title.is_some()
            && should_refresh_now_playing_toast_after_volume_change(source, music_volume)
        {
            self.now_playing_started_at = Some(web_time::Instant::now());
        }
    }
    pub fn set_audio_output_catalog(&mut self, catalog: crate::client::audio::AudioOutputCatalog) {
        let choices = if catalog.supported {
            let mut choices = Vec::with_capacity(catalog.devices.len() + 1);
            choices.push(el_gui::SoundOutputChoice {
                handle: String::new(),
                name: crate::ui::i18n::translate("options.audioDevice.default").to_string(),
            });
            choices.extend(
                catalog
                    .devices
                    .iter()
                    .map(|device| el_gui::SoundOutputChoice {
                        handle: device.handle.clone(),
                        name: device.name.clone(),
                    }),
            );
            choices
        } else {
            Vec::new()
        };
        let preferred = catalog.preferred.clone();
        self.audio_output_catalog = Some(catalog);
        if let Some(ActiveScreen::SoundOptions(screen)) = self.active_screen.as_mut() {
            screen.set_audio_output_choices(choices, &preferred);
            self.mark_screen_dirty();
        }
    }
    pub fn take_directional_audio_change(&mut self) -> Option<bool> {
        self.pending_directional_audio.take()
    }
    pub fn take_audio_output_refresh_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_audio_output_refresh)
    }
    pub fn take_audio_output_selection_request(
        &mut self,
    ) -> Option<(u64, crate::client::audio::AudioOutputSelection)> {
        self.pending_audio_output_selection.take()
    }
    fn queue_audio_output_selection(&mut self, handle: String) {
        let Some(catalog) = self.audio_output_catalog.as_ref() else {
            return;
        };
        if !catalog.supported {
            return;
        }
        let selection = if handle.is_empty() {
            crate::client::audio::AudioOutputSelection::Default
        } else {
            crate::client::audio::AudioOutputSelection::Device(handle)
        };
        self.pending_audio_output_selection = Some((catalog.revision, selection));
    }
    pub fn set_audio_device_preference(&mut self, preferred: String) {
        if self.extra_options.get("soundDevice") == Some(&preferred) {
            return;
        }
        self.extra_options
            .insert("soundDevice".to_string(), preferred);
        self.save_options();
    }
    pub fn take_pending_ui_sounds(&mut self) -> Vec<el_gui::components::WidgetClickSound> {
        let mut sounds = std::mem::take(&mut self.pending_ui_sounds);
        sounds.extend(el_gui::components::click_sound_sink::take_all());
        sounds
    }
    pub fn take_terrain_mesh_rebuild_request(&mut self) -> bool {
        std::mem::take(&mut self.terrain_mesh_rebuild_pending)
    }
    pub fn configure_game_directories(
        &mut self,
        game_directory: std::path::PathBuf,
        resource_pack_directory: Option<std::path::PathBuf>,
    ) {
        self.options_path = game_directory.join("options.txt");
        self.command_history_path = game_directory.join("command_history.txt");
        self.resource_pack_directory = resource_pack_directory;
    }
    pub fn reload_options_from_disk(&mut self) {
        let previous_font_options = self.font_options();
        self.load_options();
        if self.font_options() != previous_font_options {
            self.pending_font_reload = true;
        }
        self.load_command_history();
    }
    fn resource_pack_directory(&self) -> std::path::PathBuf {
        self.resource_pack_directory.clone().unwrap_or_else(|| {
            crate::resources::pack_loader::resource_packs_dir(&self.options_path)
        })
    }
    pub fn reload_resource_consumers(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: ClientResourceManager,
        target_format: wgpu::TextureFormat,
    ) -> Result<(), ResourceError> {
        let renderer = GuiRenderer::new(
            device,
            queue,
            texture_bind_group_layout,
            &resource_manager,
            target_format,
        )?;
        let font = FontRenderer::load(
            device,
            queue,
            texture_bind_group_layout,
            &resource_manager,
            self.font_options(),
        )?;
        self.renderer = renderer;
        self.font = std::sync::Arc::new(font);
        self.hud.reload_locator_waypoint_styles(&resource_manager);
        let persisted_language_code = match self.extra_options.get("lang") {
            Some(code) if !code.trim().is_empty() => code.trim().to_string(),
            _ => {
                #[cfg(target_arch = "wasm32")]
                let source = String::new();
                #[cfg(not(target_arch = "wasm32"))]
                let source = std::fs::read_to_string(&self.options_path).unwrap_or_default();
                crate::ui::i18n::active_language_code_from_options(&source).to_string()
            }
        };
        crate::ui::i18n::reload_from_resource_manager(&resource_manager, &persisted_language_code);
        self.resource_manager = resource_manager;
        Ok(())
    }
    fn advance_realtime_delta_ticks(&mut self) -> f32 {
        let now = web_time::Instant::now();
        let realtime_delta_ticks = now
            .duration_since(self.last_realtime_delta_at)
            .as_secs_f32()
            * 20.0;
        self.last_realtime_delta_at = now;
        if realtime_delta_ticks > 7.0 {
            0.5
        } else {
            realtime_delta_ticks
        }
    }
    fn advance_menu_panorama(&mut self, realtime_delta_ticks: f32) -> f32 {
        let panorama_speed = self.option_float("panoramaSpeed", 1.0).clamp(0.0, 1.0);
        self.panorama_spin =
            wrap_degrees(self.panorama_spin + realtime_delta_ticks * panorama_speed * 0.1);
        -self.panorama_spin
    }
    pub fn is_screen_active(&self) -> bool {
        self.active_screen.is_some()
    }
    pub fn is_text_input_active(&self) -> bool {
        match &self.active_screen {
            Some(
                ActiveScreen::DirectConnect(_)
                | ActiveScreen::ManageServer(_)
                | ActiveScreen::Chat { .. }
                | ActiveScreen::InBedChat(_),
            ) => true,
            Some(ActiveScreen::Container(screen)) => screen
                .creative_search_box
                .as_ref()
                .is_some_and(|b| b.is_visible() && b.is_focused()),
            _ => false,
        }
    }
    pub fn inject_android_text(&mut self, text: &str) {
        let is_backspace = text == "\x08";
        let Some(screen) = self.active_screen.as_mut() else {
            return;
        };
        match screen {
            ActiveScreen::DirectConnect(screen) => {
                if is_backspace {
                    use el_gui::framework::screen::Screen;
                    screen.key_pressed(el_gui::input::events::KeyEvent::new(
                        el_gui::input::events::KeyEvent::KEY_BACKSPACE,
                        0,
                        0,
                    ));
                } else {
                    use el_gui::framework::screen::Screen;
                    for ch in text.chars() {
                        if !ch.is_control() {
                            screen.char_typed(el_gui::input::events::CharacterEvent::new(ch));
                        }
                    }
                }
            }
            ActiveScreen::ManageServer(screen) => {
                if is_backspace {
                    use el_gui::framework::screen::Screen;
                    screen.key_pressed(el_gui::input::events::KeyEvent::new(
                        el_gui::input::events::KeyEvent::KEY_BACKSPACE,
                        0,
                        0,
                    ));
                } else {
                    use el_gui::framework::screen::Screen;
                    for ch in text.chars() {
                        if !ch.is_control() {
                            screen.char_typed(el_gui::input::events::CharacterEvent::new(ch));
                        }
                    }
                }
            }
            ActiveScreen::Chat { input } => {
                if is_backspace {
                    if !input.is_empty() {
                        input.pop();
                    }
                } else {
                    for ch in text.chars() {
                        if !ch.is_control() {
                            input.push(ch);
                        }
                    }
                }
            }
            ActiveScreen::InBedChat(screen) => {
                let input = &mut screen.input;
                if is_backspace {
                    if !input.is_empty() {
                        input.pop();
                    }
                } else {
                    for ch in text.chars() {
                        if !ch.is_control() {
                            input.push(ch);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fn create_win_screen(&self, poem: bool) -> Result<WinScreen, String> {
        let load_text = |resource: &str, fallback: &str| -> String {
            let Some((namespace, path)) = resource.split_once(':') else {
                log::error!("Invalid WinScreen resource identifier: {resource}");
                return fallback.to_owned();
            };
            let id = ResourceLocation::new(namespace, path);
            match self.resource_manager.read_resource_bytes(&id) {
                Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
                Err(error) => {
                    log::error!("Couldn't load WinScreen resource {resource}: {error}");
                    fallback.to_owned()
                }
            }
        };
        let credits = load_text(CREDITS_RESOURCE, "[]");
        let end_poem = poem.then(|| load_text(END_POEM_RESOURCE, ""));
        let postcredits = poem.then(|| load_text(POSTCREDITS_RESOURCE, ""));
        let resources = WinScreenResources {
            credits_json: &credits,
            end_poem: end_poem.as_deref(),
            postcredits: postcredits.as_deref(),
        };
        let screen = match WinScreen::from_resources(
            poem,
            &self.player_username,
            resources,
            self.font.as_ref(),
        ) {
            Ok(screen) => screen,
            Err(error) => {
                log::error!("Couldn't parse WinScreen credits: {error}");
                WinScreen::from_resources(
                    poem,
                    &self.player_username,
                    WinScreenResources {
                        credits_json: "[]",
                        end_poem: end_poem.as_deref(),
                        postcredits: postcredits.as_deref(),
                    },
                    self.font.as_ref(),
                )
                .map_err(|fallback_error| fallback_error.to_string())?
            }
        };
        Ok(screen)
    }
    pub fn open_win_screen(&mut self, poem: bool) -> Result<(), String> {
        let screen = self.create_win_screen(poem)?;
        if poem {
            self.menu_parent_stack.clear();
            self.code_of_conduct_parent = None;
            self.pending_win_screen_respawn_request = false;
        }
        self.active_screen = Some(ActiveScreen::WinScreen {
            screen,
            parent: None,
        });
        self.mark_screen_dirty();
        Ok(())
    }
    fn finish_win_screen(&mut self, poem: bool, parent: Option<Box<ActiveScreen>>) {
        self.mark_screen_dirty();
        if poem {
            self.active_screen = None;
            self.pending_win_screen_respawn_request = true;
        } else {
            self.active_screen = parent.map(|parent| *parent);
        }
    }
    pub fn take_win_screen_respawn_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_win_screen_respawn_request)
    }
    pub fn open_death_screen(
        &mut self,
        cause_of_death: Option<String>,
        hardcore: bool,
        score: Option<i32>,
    ) {
        self.menu_parent_stack.clear();
        self.code_of_conduct_parent = None;
        self.pending_death_screen_respawn_request = false;
        let screen = DeathScreen::new(
            cause_of_death,
            hardcore,
            score,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        self.active_screen = Some(ActiveScreen::DeathScreen(screen));
        self.mark_screen_dirty();
    }
    pub fn take_death_screen_respawn_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_death_screen_respawn_request)
    }
    pub fn take_statistics_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_statistics_request)
    }
    fn open_game_rules_screen(&mut self, in_world: bool) {
        let translated = crate::ui::i18n::translate("editGamerule.title");
        let title = if translated == "editGamerule.title" {
            "Game Rules".to_owned()
        } else {
            translated.into_owned()
        };
        let mut screen = GameRulesScreen::new(
            title,
            Vec::new(),
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        screen.set_in_world(in_world);
        screen.set_loading(true);
        self.active_screen = Some(ActiveScreen::GameRules(screen));
        self.pending_game_rule_values_request = true;
        self.mark_screen_dirty();
    }
    fn supports_game_rule_editor(&self) -> bool {
        self.game_rule_editor_supported
    }
    pub fn set_game_rule_editor_supported(&mut self, supported: bool) {
        self.game_rule_editor_supported = supported;
    }
    pub fn take_game_rule_values_request(&mut self) -> bool {
        std::mem::take(&mut self.pending_game_rule_values_request)
    }
    pub fn take_game_rule_edits(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.pending_game_rule_edits)
    }
    pub fn take_sign_update_request(&mut self) -> Option<([i32; 3], bool, [String; 4])> {
        self.pending_sign_update.take()
    }
    pub fn update_game_rule_values(&mut self, values: &std::collections::HashMap<String, String>) {
        let rows = menu_data::game_rule_rows(values);
        if let Some(ActiveScreen::GameRules(screen)) = self.active_screen.as_mut() {
            screen.replace_rows(rows);
            self.mark_screen_dirty();
        }
    }
    pub fn take_seen_advancement_actions(&mut self) -> Vec<SeenAdvancementsAction> {
        std::mem::take(&mut self.pending_seen_advancement_actions)
    }
    pub fn update_statistics(&mut self, packet: &el_protocol_backend::BackendAwardStats) {
        menu_data::merge_awarded_stats(&mut self.statistics_values, packet);
        let data = menu_data::statistics_data(&self.statistics_values);
        if let Some(ActiveScreen::Stats(screen)) = self.active_screen.as_mut() {
            screen.replace_data(data);
        }
        self.mark_screen_dirty();
    }
    pub fn update_advancements(&mut self, packet: &el_protocol_backend::BackendUpdateAdvancements) {
        let update = menu_data::advancements_update(packet);
        let mut model = AdvancementsScreen::new(
            self.advancements_snapshot.clone(),
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        model.apply_update(update.clone());
        self.advancements_snapshot = model.snapshot();
        if let Some(ActiveScreen::Advancements(screen)) = self.active_screen.as_mut() {
            screen.apply_update(update);
            self.advancements_snapshot = screen.snapshot();
        }
        self.mark_screen_dirty();
    }
    pub fn set_selected_advancements_tab(&mut self, tab: Option<String>) {
        self.selected_advancements_tab = tab.clone();
        if let Some(ActiveScreen::Advancements(screen)) = self.active_screen.as_mut() {
            match tab.as_deref() {
                Some(tab) => {
                    screen.set_selected_tab(tab);
                }
                None => screen.clear_selected_tab(),
            }
        }
        self.mark_screen_dirty();
    }
    pub fn set_server_links(&mut self, links: Vec<(String, String)>) {
        let bounded = links
            .into_iter()
            .take(128)
            .map(|(label, url)| {
                (
                    label.chars().take(256).collect(),
                    url.chars().take(2048).collect(),
                )
            })
            .collect::<Vec<_>>();
        if self.server_links != bounded {
            self.server_links = bounded;
            self.mark_screen_dirty();
        }
    }
    pub fn set_pause_screen_dialog(
        &mut self,
        dialog_kind: String,
        dialog_nbt: Option<serde_json::Value>,
        requires_server_links: bool,
    ) {
        self.pause_screen_dialog = Some((dialog_kind, dialog_nbt, requires_server_links));
        self.mark_screen_dirty();
    }
    fn pause_screen_addition(&self) -> (bool, Option<String>) {
        pause_dialog_addition(
            self.pause_screen_dialog.as_ref(),
            !self.server_links.is_empty(),
        )
    }
    pub fn show_server_dialog(
        &mut self,
        dialog_kind: Option<String>,
        dialog_nbt: Option<serde_json::Value>,
        in_world: bool,
    ) {
        let previous = self.active_screen.take();
        let (parent, inherited_world) = match previous {
            Some(ActiveScreen::ServerDialog {
                parent, in_world, ..
            }) => (parent, in_world),
            Some(screen) => {
                let inherited_world = screen.is_in_world();
                (Some(Box::new(screen)), inherited_world)
            }
            None => (None, in_world),
        };
        let model = server_dialog_model(
            dialog_kind.as_deref(),
            dialog_nbt.as_ref(),
            &self.server_links,
        );
        let screen = ServerDialogScreen::new(
            model,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        let mut screen = screen;
        screen.set_clipboard(self.clipboard.clone());
        screen.set_background_route(if inherited_world {
            ServerDialogBackground::InWorldDialogBase
        } else {
            ServerDialogBackground::MenuDialogBase
        });
        self.active_screen = Some(ActiveScreen::ServerDialog {
            screen,
            parent,
            in_world: inherited_world,
        });
        self.mark_screen_dirty();
    }
    pub fn open_sign_editor(
        &mut self,
        pos: [i32; 3],
        is_front_text: bool,
        initial_lines: [String; 4],
    ) {
        let model = sign_editor_dialog_model(pos, is_front_text, initial_lines);
        let previous = self.active_screen.take();
        let (parent, in_world) = match previous {
            Some(ActiveScreen::ServerDialog {
                parent, in_world, ..
            }) => (parent, in_world),
            Some(screen) => {
                let in_world = screen.is_in_world();
                (Some(Box::new(screen)), in_world)
            }
            None => (None, true),
        };
        let mut screen = ServerDialogScreen::new(
            model,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        screen.set_clipboard(self.clipboard.clone());
        screen.set_background_route(ServerDialogBackground::TransparentSignInWorld);
        self.active_screen = Some(ActiveScreen::ServerDialog {
            screen,
            parent,
            in_world,
        });
        self.mark_screen_dirty();
    }
    pub fn clear_server_dialog(&mut self) {
        match self.active_screen.take() {
            Some(ActiveScreen::ServerDialog { parent, .. }) => {
                self.active_screen = parent.map(|parent| *parent);
                self.mark_screen_dirty();
            }
            other => self.active_screen = other,
        }
    }
    pub fn open_advancements_screen(&mut self) {
        let mut screen = AdvancementsScreen::new(
            self.advancements_snapshot.clone(),
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        )
        .with_icon_renderer(std::sync::Arc::new(ClientAdvancementIconRenderer));
        screen.set_in_world(true);
        if let Some(tab) = self.selected_advancements_tab.as_deref() {
            screen.set_selected_tab(tab);
        }
        self.active_screen = Some(ActiveScreen::Advancements(screen));
        self.mark_screen_dirty();
    }
    pub fn toggle_advancements_screen(&mut self) {
        match self.active_screen.take() {
            Some(ActiveScreen::Advancements(mut screen)) => {
                screen.removed();
                self.collect_seen_advancement_actions(&mut screen);
                self.active_screen = self.menu_parent_stack.pop();
                self.mark_screen_dirty();
            }
            Some(screen) => self.active_screen = Some(screen),
            None => self.open_advancements_screen(),
        }
    }
    fn collect_seen_advancement_actions(&mut self, screen: &mut AdvancementsScreen) {
        for action in screen.take_seen_advancement_actions() {
            if let SeenAdvancementsAction::OpenedTab(tab) = &action {
                self.selected_advancements_tab = Some(tab.clone());
            }
            self.pending_seen_advancement_actions.push(action);
        }
    }
    fn open_stats_screen(&mut self) {
        let mut screen = StatsScreen::new(
            menu_data::statistics_data(&self.statistics_values),
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        screen.set_in_world(true);
        self.active_screen = Some(ActiveScreen::Stats(screen));
        self.pending_statistics_request = true;
        self.mark_screen_dirty();
    }
    pub fn close_death_screen_after_respawn(&mut self) {
        let should_close = matches!(
            self.active_screen.as_ref(),
            Some(ActiveScreen::DeathScreen(_))
                | Some(ActiveScreen::Confirm(ConfirmScreen {
                    context: ConfirmContext::DeathScreen,
                    ..
                }))
        );
        if should_close {
            self.active_screen = None;
            self.mark_screen_dirty();
        }
    }
    #[cfg(target_os = "android")]
    pub fn handle_android_key_action(
        &mut self,
        action: crate::input::android_touch::AndroidKeyAction,
        player: &mut LocalPlayer,
    ) -> bool {
        if action == crate::input::android_touch::AndroidKeyAction::Escape {
            match self.active_screen.take() {
                Some(ActiveScreen::WinScreen { screen, parent }) => {
                    self.finish_win_screen(screen.poem(), parent);
                    return true;
                }
                other => self.active_screen = other,
            }
        }
        let (input, is_bed) = match self.active_screen.take() {
            Some(ActiveScreen::Chat { input }) => (input, false),
            Some(ActiveScreen::InBedChat(screen)) => (screen.input, true),
            other => {
                self.active_screen = other;
                return false;
            }
        };
        match action {
            crate::input::android_touch::AndroidKeyAction::Enter => {
                let message = screen::normalize_chat_message(&input);
                self.chat_component.discard_draft();
                self.chat_component.reset_chat_scroll();
                crate::hide_android_keyboard();
                if !message.is_empty() {
                    self.submit_chat(message, player);
                }
                true
            }
            crate::input::android_touch::AndroidKeyAction::Escape => {
                self.server_chat_suggestions.clear();
                self.server_suggestion_input = None;
                self.chat_suggestion_index = 0;
                self.chat_tab_cycles = false;
                if self.option_bool("saveChatDrafts", false) && !input.trim().is_empty() {
                    self.chat_component.save_as_draft(input.clone());
                } else {
                    self.chat_component.discard_draft();
                }
                self.chat_component.reset_chat_scroll();
                if is_bed {
                    self.pending_unattended_chat_commands
                        .push("/stop_sleeping".to_string());
                }
                crate::hide_android_keyboard();
                true
            }
        }
    }
    pub fn begin_level_loading_screen(&mut self) {
        if self.is_level_loading_screen_active() {
            return;
        }
        let preserved_chat = match self.active_screen.take() {
            Some(ActiveScreen::Chat { input }) => Some(PreservedLevelLoadingChat {
                input,
                is_draft: self.chat_is_draft,
            }),
            Some(ActiveScreen::InBedChat(screen)) => Some(PreservedLevelLoadingChat {
                input: screen.input,
                is_draft: screen.is_draft,
            }),
            _ => None,
        };
        self.active_screen = Some(ActiveScreen::LevelLoading(LevelLoadingScreen {
            preserved_chat,
        }));
        self.code_of_conduct_parent = None;
        self.stashed_multiplayer_screen = None;
        self.connecting_server_ip = None;
    }
    pub fn finish_level_loading_screen(&mut self) {
        let Some(screen) = self.active_screen.take() else {
            return;
        };
        let ActiveScreen::LevelLoading(level_loading) = screen else {
            self.active_screen = Some(screen);
            return;
        };
        if let Some(chat) = level_loading.preserved_chat {
            self.chat_cursor = chat.input.chars().count();
            self.chat_input_display_pos = 0;
            self.chat_selection_anchor = None;
            self.chat_suggestion_index = 0;
            self.chat_tab_cycles = false;
            self.chat_is_draft = chat.is_draft;
            self.active_screen = Some(ActiveScreen::Chat { input: chat.input });
        }
    }
    pub fn is_level_loading_screen_active(&self) -> bool {
        matches!(self.active_screen, Some(ActiveScreen::LevelLoading(_)))
    }
    pub fn realms_world_template_texture(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        id: &str,
        image: Option<&str>,
    ) -> ResourceLocation {
        self.realms_texture_manager.world_template(
            &mut self.renderer,
            device,
            queue,
            texture_bind_group_layout,
            id,
            image,
        )
    }
    pub fn is_active_screen_in_world(&self) -> bool {
        self.active_screen
            .as_ref()
            .map(|s| s.is_in_world())
            .unwrap_or(true)
    }
    pub fn active_screen_mut(&mut self) -> Option<&mut ActiveScreen> {
        self.mark_screen_dirty();
        self.active_screen.as_mut()
    }
    pub fn active_screen(&self) -> Option<&ActiveScreen> {
        self.active_screen.as_ref()
    }
    pub fn active_container_id(&self) -> Option<i32> {
        match self.active_screen.as_ref() {
            Some(ActiveScreen::Container(screen)) => Some(screen.container_id),
            _ => None,
        }
    }
    pub fn menu_background_blur_radius(&self) -> f32 {
        let blur_radius =
            menu_blur_radius_from_stored_option(self.option_float("menuBackgroundBlurriness", 0.5));
        menu_background_blur_radius_for(self.active_screen.as_ref(), blur_radius)
    }
    pub fn get_gui_scale_factor(&self, screen_w: f32, screen_h: f32) -> u32 {
        let max_scale_x = (screen_w / 320.0).floor() as u32;
        let max_scale_y = (screen_h / 240.0).floor() as u32;
        let max_scale = max_scale_x.min(max_scale_y).max(1);
        let scale = if self.gui_scale_setting == 0 {
            max_scale
        } else {
            self.gui_scale_setting.min(max_scale)
        };
        if self.font_options().uniform && scale % 2 != 0 {
            scale + 1
        } else {
            scale
        }
    }
    pub fn cycle_gui_scale(&mut self) {
        self.gui_scale_setting = (self.gui_scale_setting + 1) % 5;
        self.update_logical_mouse();
        self.save_options();
    }
    pub fn update_logical_mouse(&mut self) {
        if let Some((x, y)) = self.autoshot_logical_mouse {
            self.mouse_x = x;
            self.mouse_y = y;
            return;
        }
        let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
        self.mouse_x = self.physical_mouse_x / scale;
        self.mouse_y = self.physical_mouse_y / scale;
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn sync_cursor_after_warp(
        &mut self,
        position: winit::dpi::PhysicalPosition<f64>,
        warped: bool,
    ) {
        if warped {
            self.physical_mouse_x = position.x as f32;
            self.physical_mouse_y = position.y as f32;
            self.update_logical_mouse();
        }
    }
    pub fn load_options(&mut self) {
        self.pending_window_fullscreen = None;
        #[cfg(target_arch = "wasm32")]
        let content = web_sys::window()
            .and_then(|w| w.local_storage().ok().flatten())
            .and_then(|s| s.get_item("options.txt").ok().flatten());
        #[cfg(not(target_arch = "wasm32"))]
        let content = std::fs::read_to_string(&self.options_path)
            .ok()
            .or_else(|| std::fs::read_to_string("options.txt").ok());
        if let Some(content_str) = content {
            for line in content_str.lines() {
                let Some((raw_key, raw_val)) = line.split_once(':') else {
                    continue;
                };
                let key = raw_key.trim();
                let val = raw_val.trim();
                match key {
                    "guiScale" => {
                        if let Ok(scale) = val.parse::<u32>() {
                            self.gui_scale_setting = scale.min(4);
                        } else if let Ok(val_f) = val.parse::<f32>() {
                            self.gui_scale_setting = ((val_f * 4.0).round() as u32).min(4);
                        }
                    }
                    "fov" => {
                        if let Ok(fov) = val.parse::<f32>() {
                            self.fov_setting = fov_slider_from_java_option(fov);
                        }
                    }
                    "sensitivity" | "mouseSensitivity" => {
                        if let Some(sensitivity) = parse_mouse_sensitivity_option(val) {
                            self.sensitivity_setting = sensitivity;
                        }
                    }
                    "mouseWheelSensitivity" => {
                        if let Ok(sens) = val.parse::<f32>() {
                            self.mouse_wheel_sensitivity = sens.clamp(0.01, 10.0);
                        }
                    }
                    "invertXMouse" | "invertMouseX" => {
                        self.invert_mouse_x = val == "true";
                    }
                    "invertYMouse" | "invertMouseY" => {
                        self.invert_mouse_y = val == "true";
                    }
                    "bobView" | "viewBobbing" => self.view_bobbing = val != "false",
                    "damageTiltStrength" => {
                        if let Ok(tilt) = val.parse::<f64>() {
                            self.damage_tilt_strength = tilt.clamp(0.0, 1.0);
                        }
                    }
                    "toggleCrouch" => {
                        self.toggle_crouch = val == "true";
                        self.extra_options.insert(key.to_string(), val.to_string());
                    }
                    "toggleSprint" => {
                        self.toggle_sprint = val == "true";
                        self.extra_options.insert(key.to_string(), val.to_string());
                    }
                    "toggleAttack" => {
                        self.toggle_attack = val == "true";
                        self.extra_options.insert(key.to_string(), val.to_string());
                    }
                    "toggleUse" => {
                        self.toggle_use = val == "true";
                        self.extra_options.insert(key.to_string(), val.to_string());
                    }
                    "fullscreen" => {
                        let enabled = val == "true";
                        self.extra_options.insert(key.to_string(), val.to_string());
                        self.pending_window_fullscreen = Some(enabled);
                    }
                    "fullscreenOption" => {
                        self.extra_options.remove(key);
                    }
                    "smoothCamera" => self.smooth_camera = val == "true",
                    "discreteMouseScroll" => self.discrete_mouse_scroll = val == "true",
                    "allowCursorChanges" => self.allow_cursor_changes = val != "false",
                    "lastServer" | "lastMpIp" => {
                        self.extra_options
                            .insert("lastMpIp".to_string(), val.to_string());
                    }
                    _ => {
                        self.extra_options.insert(key.to_string(), val.to_string());
                    }
                }
            }
        }
    }
    pub fn save_options(&self) {
        let sensitivity = format_mouse_sensitivity_option(self.sensitivity_setting);
        let mut content = format!(
            concat!(
                "guiScale:{}\n",
                "fov:{}\n",
                "sensitivity:{}\n",
                "mouseWheelSensitivity:{}\n",
                "invertXMouse:{}\n",
                "invertYMouse:{}\n",
                "bobView:{}\n",
                "damageTiltStrength:{}\n",
                "toggleCrouch:{}\n",
                "toggleSprint:{}\n",
                "toggleAttack:{}\n",
                "toggleUse:{}\n",
                "smoothCamera:{}\n",
                "discreteMouseScroll:{}\n",
                "allowCursorChanges:{}\n",
            ),
            self.gui_scale_setting,
            java_fov_option_from_slider(self.fov_setting),
            sensitivity,
            self.mouse_wheel_sensitivity,
            self.invert_mouse_x,
            self.invert_mouse_y,
            self.view_bobbing,
            self.damage_tilt_strength,
            self.toggle_crouch,
            self.toggle_sprint,
            self.toggle_attack,
            self.toggle_use,
            self.smooth_camera,
            self.discrete_mouse_scroll,
            self.allow_cursor_changes
        );
        for (k, v) in &self.extra_options {
            if matches!(
                k.as_str(),
                "toggleCrouch" | "toggleSprint" | "toggleAttack" | "toggleUse"
            ) {
                continue;
            }
            let key = if k == "lastMpIp" {
                "lastServer"
            } else {
                k.as_str()
            };
            content.push_str(&format!("{}:{}\n", key, v));
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(storage) = web_sys::window().and_then(|w| w.local_storage().ok().flatten())
            {
                let _ = storage.set_item("options.txt", &content);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = std::fs::write(&self.options_path, &content);
            let _ = std::fs::write("options.txt", &content);
        }
    }
    fn option_bool(&self, key: &str, default: bool) -> bool {
        self.extra_options
            .get(key)
            .map(|value| value == "true")
            .unwrap_or(default)
    }
    pub fn ambient_occlusion_enabled(&self) -> bool {
        ambient_occlusion_enabled_from_options(&self.extra_options)
    }
    pub fn auto_jump_enabled(&self) -> bool {
        self.option_bool("autoJump", false)
    }
    pub fn high_contrast_block_outline_enabled(&self) -> bool {
        self.option_bool("highContrastBlockOutline", false)
    }
    fn option_float(&self, key: &str, default: f32) -> f32 {
        self.extra_options
            .get(key)
            .and_then(|value| value.parse::<f32>().ok())
            .unwrap_or(default)
    }
    pub fn enchantment_glint_options(&self) -> (f32, f32) {
        fn value(
            options: &std::collections::HashMap<String, String>,
            name: &str,
            fallback: f32,
        ) -> f32 {
            options
                .get(name)
                .and_then(|value| value.parse::<f32>().ok())
                .filter(|value| value.is_finite() && (0.0..=1.0).contains(value))
                .unwrap_or(fallback)
        }
        (
            value(&self.extra_options, "glintSpeed", 0.5),
            value(&self.extra_options, "glintStrength", 0.75),
        )
    }
    pub(crate) fn client_information_language(&self) -> &str {
        client_information_language_option(self.extra_options.get("lang").map(String::as_str))
    }
    fn client_information_view_distance(&self) -> u8 {
        self.extra_options
            .get("renderDistance")
            .and_then(|value| value.parse::<u8>().ok())
            .filter(|distance| (2..=32).contains(distance))
            .unwrap_or(12)
    }
    fn client_information_main_hand_left(&self) -> bool {
        matches!(
            self.extra_options.get("mainHand").map(String::as_str),
            Some("left")
        )
    }
    pub(crate) fn client_information_allows_listing(&self) -> bool {
        client_information_allows_listing_option(
            self.extra_options
                .get("allowServerListing")
                .map(String::as_str),
        )
    }
    fn set_client_information_allows_listing(&mut self, enabled: bool) {
        self.extra_options.insert(
            "allowServerListing".to_string(),
            if enabled { "true" } else { "false" }.to_string(),
        );
        self.save_options();
        self.mark_screen_dirty();
    }
    fn client_information_model_customisation(&self) -> u8 {
        let mut mask = 0u8;
        let enabled = |key: &str| -> bool {
            match self.extra_options.get(key).map(String::as_str) {
                Some("true") => true,
                Some("false") => false,
                _ => true,
            }
        };
        if enabled("modelPart_cape") {
            mask |= 1 << 0;
        }
        if enabled("modelPart_jacket") {
            mask |= 1 << 1;
        }
        if enabled("modelPart_left_sleeve") {
            mask |= 1 << 2;
        }
        if enabled("modelPart_right_sleeve") {
            mask |= 1 << 3;
        }
        if enabled("modelPart_left_pants_leg") {
            mask |= 1 << 4;
        }
        if enabled("modelPart_right_pants_leg") {
            mask |= 1 << 5;
        }
        if enabled("modelPart_hat") {
            mask |= 1 << 6;
        }
        mask
    }
    fn load_title_splash(&self) -> Option<String> {
        let content_opt = self
            .resource_manager
            .read_resource_bytes(&ResourceLocation::new("minecraft", "texts/splashes.txt"))
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .or_else(|| {
                let path = self
                    .resource_manager
                    .assets_root()
                    .join("minecraft")
                    .join("texts")
                    .join("splashes.txt");
                std::fs::read_to_string(path).ok()
            });
        let mut splashes: Vec<String> = content_opt
            .as_deref()
            .unwrap_or("")
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .filter(|line| java_string_hash(line) != 125780783)
            .map(ToOwned::to_owned)
            .collect();
        if splashes.is_empty() {
            splashes = vec![
                "Minecraft!".to_string(),
                "Eltharn!".to_string(),
                "100% pure!".to_string(),
                "May contain nuts!".to_string(),
                "Also try Terraria!".to_string(),
                "User-defined!".to_string(),
                "Awesome!".to_string(),
                "Made in Rust!".to_string(),
                "WebGL 2 / WGPU!".to_string(),
            ];
        }
        let nanos = web_time::SystemTime::now()
            .duration_since(web_time::UNIX_EPOCH)
            .map(|duration| duration.subsec_nanos() as usize)
            .unwrap_or(0);
        splashes.get(nanos % splashes.len()).cloned()
    }
    fn apply_mouse_settings(&mut self, screen: &el_gui::MouseSettingsScreen) {
        for opt in &screen.options {
            if opt.key == "sensitivity" {
                self.sensitivity_setting = opt.value;
            } else if opt.key == "mouseWheelSensitivity" {
                self.mouse_wheel_sensitivity = opt.value as f32;
            } else if opt.key == "discreteMouseScroll" {
                self.discrete_mouse_scroll = opt.value > 0.5;
            } else if opt.key == "invertMouseX" {
                self.invert_mouse_x = opt.value > 0.5;
            } else if opt.key == "invertMouseY" {
                self.invert_mouse_y = opt.value > 0.5;
            } else if opt.key == "allowCursorChanges" {
                self.allow_cursor_changes = opt.value > 0.5;
            }
        }
    }
    fn apply_video_settings(&mut self, video_screen: &VideoSettingsScreen) {
        self.apply_video_settings_options(&video_screen.options);
        if video_screen.has_fullscreen_video_mode_service() {
            let preferred = video_screen
                .preferred_fullscreen_video_mode()
                .map(FullscreenVideoMode::serialize);
            if self.extra_options.get("fullscreenResolution").cloned() != preferred {
                if let Some(preferred) = preferred {
                    self.extra_options
                        .insert("fullscreenResolution".to_string(), preferred);
                } else {
                    self.extra_options.remove("fullscreenResolution");
                }
                self.fullscreen_video_mode_dirty = true;
            }
        }
    }
    fn video_settings_screen(&self, in_world: bool) -> VideoSettingsScreen {
        VideoSettingsScreen::new_with_video_modes(
            in_world,
            &self.extra_options,
            self.gui_scale_setting,
            &self.fullscreen_video_modes,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        )
    }
    fn apply_video_settings_options(
        &mut self,
        options: &[el_gui::settings::video_settings::VideoSettingsOption],
    ) {
        let old_ambient_occlusion = self.ambient_occlusion_enabled();
        for opt in options {
            match opt.key.as_str() {
                "guiScale" => {}
                "ambientOcclusion" | "ao" => {
                    let bool_str = if opt.value > 0.5 { "true" } else { "false" };
                    self.extra_options
                        .insert("ao".to_string(), bool_str.to_string());
                    self.extra_options.remove("ambientOcclusion");
                }
                "enableVsync"
                | "fullscreen"
                | "exclusiveFullscreen"
                | "entityShadows"
                | "improvedTransparency"
                | "showAutosaveIndicator"
                | "vignette"
                | "cutoutLeaves" => {
                    let bool_str = if opt.value > 0.5 { "true" } else { "false" };
                    if opt.key == "fullscreen" {
                        let was_enabled = self
                            .extra_options
                            .get("fullscreen")
                            .is_some_and(|value| value == "true");
                        let enabled = opt.value > 0.5;
                        if was_enabled != enabled {
                            self.pending_window_fullscreen = Some(enabled);
                        }
                    }
                    if opt.key == "enableVsync" {
                        log::debug!(
                            "[Render] enableVsync option -> {bool_str} (was {:?})",
                            self.extra_options.get("enableVsync")
                        );
                    }
                    self.extra_options
                        .insert(opt.key.clone(), bool_str.to_string());
                }
                "inactivityFpsLimit" => {
                    let value = if opt.value > 0.5 { "afk" } else { "minimized" };
                    self.extra_options
                        .insert(opt.key.clone(), value.to_string());
                }
                "graphicsPreset" => {
                    let value = match opt.value.round() as i32 {
                        0 => "fast",
                        1 => "fancy",
                        2 => "fabulous",
                        _ => "custom",
                    };
                    self.extra_options
                        .insert(opt.key.clone(), value.to_string());
                }
                "cloudStatus" | "renderClouds" => {
                    let value = match opt.value.round() as i32 {
                        0 => "false",
                        1 => "fast",
                        _ => "true",
                    };
                    self.extra_options
                        .insert("renderClouds".to_string(), value.to_string());
                    self.extra_options.remove("cloudStatus");
                }
                "framerateLimit" => {
                    self.extra_options
                        .insert("maxFps".to_string(), (opt.value.round() as i32).to_string());
                    self.extra_options.remove("framerateLimit");
                }
                "biomeBlendRadius"
                | "renderDistance"
                | "simulationDistance"
                | "cloudRange"
                | "mipmapLevels"
                | "menuBackgroundBlurriness"
                | "maxAnisotropyBit"
                | "weatherRadius" => {
                    self.extra_options
                        .insert(opt.key.clone(), (opt.value.round() as i32).to_string());
                }
                "entityDistanceScaling" | "chunkSectionFadeInTime" | "gamma" => {
                    self.extra_options
                        .insert(opt.key.clone(), opt.value.to_string());
                }
                "prioritizeChunkUpdates" | "particles" | "textureFiltering" | "attackIndicator" => {
                    self.extra_options
                        .insert(opt.key.clone(), (opt.value.round() as i32).to_string());
                }
                "preferredGraphicsBackend" => {
                    let value = match opt.value.round() as i32 {
                        1 => "opengl",
                        2 => "vulkan",
                        _ => "default",
                    };
                    self.extra_options
                        .insert(opt.key.clone(), value.to_string());
                }
                _ => {
                    self.extra_options
                        .insert(opt.key.clone(), opt.value.to_string());
                }
            }
        }
        if self.ambient_occlusion_enabled() != old_ambient_occlusion {
            self.terrain_mesh_rebuild_pending = true;
        }
    }
    fn apply_active_video_settings_live(&mut self) {
        let options = match self.active_screen.as_ref() {
            Some(ActiveScreen::Video(screen)) => screen.options.clone(),
            _ => return,
        };
        self.apply_video_settings_options(&options);
    }
    fn apply_skin_customization_settings(&mut self, screen: &el_gui::SkinCustomizationScreen) {
        for opt in &screen.options {
            if [
                "modelPart_cape",
                "modelPart_jacket",
                "modelPart_left_sleeve",
                "modelPart_right_sleeve",
                "modelPart_left_pants_leg",
                "modelPart_right_pants_leg",
                "modelPart_hat",
            ]
            .contains(&opt.key.as_str())
            {
                let bool_val = if opt.value > 0.5 { "true" } else { "false" };
                self.extra_options
                    .insert(opt.key.clone(), bool_val.to_string());
            } else if opt.key == "mainHand" {
                let val_str = match opt.value.round() as i32 {
                    0 => "left",
                    1 => "right",
                    _ => "",
                };
                self.extra_options
                    .insert(opt.key.clone(), val_str.to_string());
            } else {
                self.extra_options
                    .insert(opt.key.clone(), format!("{:.4}", opt.value));
            }
        }
    }
    fn apply_chat_options(&mut self, screen: &el_gui::ChatOptionsScreen) {
        for opt in &screen.options {
            self.apply_chat_option(opt);
        }
    }
    fn apply_chat_option(&mut self, opt: &el_gui::settings::chat_options::ChatOptionsOption) {
        {
            if opt.key == "chatColors"
                || opt.key == "chatLinks"
                || opt.key == "chatLinksPrompt"
                || opt.key == "autoSuggestions"
                || opt.key == "hideMatchedNames"
                || opt.key == "reducedDebugInfo"
                || opt.key == "onlyShowSecureChat"
                || opt.key == "saveChatDrafts"
            {
                let bool_val = if opt.value > 0.5 { "true" } else { "false" };
                self.extra_options
                    .insert(opt.key.clone(), bool_val.to_string());
            } else if opt.key == "chatVisibility" {
                let val_str = match opt.value.round() as i32 {
                    0 => "0",
                    1 => "1",
                    2 => "2",
                    _ => "0",
                };
                self.extra_options
                    .insert(opt.key.clone(), val_str.to_string());
            } else if opt.key == "narrator" {
                let val_str = match opt.value.round() as i32 {
                    0 => "0",
                    1 => "1",
                    2 => "2",
                    3 => "3",
                    _ => "0",
                };
                self.extra_options
                    .insert(opt.key.clone(), val_str.to_string());
            } else {
                let val = opt.value;
                self.extra_options
                    .insert(opt.key.clone(), format!("{:.4}", val));
            }
        }
        self.chat_component.rescale_chat();
    }
    fn apply_active_chat_options_live(&mut self) {
        if !matches!(
            self.active_screen.as_ref(),
            Some(ActiveScreen::ChatOptions(_))
        ) {
            return;
        }
        if !matches!(
            self.active_screen.as_ref(),
            Some(ActiveScreen::ChatOptions(_))
        ) {
            return;
        }
        let options = match self.active_screen.as_ref() {
            Some(ActiveScreen::ChatOptions(screen)) => screen.options.clone(),
            _ => return,
        };
        for opt in &options {
            self.apply_chat_option(opt);
        }
        self.chat_component.rescale_chat();
    }
    fn apply_active_accessibility_options_live(&mut self) {
        let snapshot = match self.active_screen.as_ref() {
            Some(ActiveScreen::AccessibilityOptions(screen)) => Some(screen.options.clone()),
            _ => None,
        };
        if let Some(snapshot) = snapshot {
            self.apply_accessibility_options_slice(&snapshot);
            self.chat_component.rescale_chat();
        }
    }
    fn apply_sound_options_settings(&mut self, screen: &el_gui::SoundOptionsScreen) {
        self.apply_sound_option_values(&screen.options);
    }
    fn apply_sound_option_values(
        &mut self,
        options: &[el_gui::settings::sound_options::SoundOptionsOption],
    ) {
        for opt in options {
            if !opt.is_available() {
                continue;
            }
            if opt.key.starts_with("soundCategory_") {
                let formatted = format!("{:.4}", opt.value);
                let changed = self
                    .extra_options
                    .get(&opt.key)
                    .map(|previous| previous != &formatted)
                    .unwrap_or(true);
                self.extra_options.insert(opt.key.clone(), formatted);
                if changed {
                    if let Some(name) = opt.key.strip_prefix("soundCategory_") {
                        if let Some(source) =
                            crate::client::sound_engine::SoundSource::from_name(name)
                        {
                            self.pending_sound_category_changes.push(source);
                        }
                    }
                }
            } else if opt.key == "showSubtitles" {
                let bool_val = if opt.value > 0.5 { "true" } else { "false" };
                self.extra_options
                    .insert(opt.key.clone(), bool_val.to_string());
            } else if opt.key == "directionalAudio" {
                let bool_val = if opt.value > 0.5 { "true" } else { "false" };
                let previous = self.extra_options.get(&opt.key).map(String::as_str);
                if previous != Some(bool_val) {
                    self.pending_directional_audio = Some(opt.value > 0.5);
                }
                self.extra_options
                    .insert(opt.key.clone(), bool_val.to_string());
            } else if opt.key == "musicFrequency" {
                let val_str = match opt.value.round() as i32 {
                    0 => "DEFAULT",
                    1 => "FREQUENT",
                    2 => "CONSTANT",
                    _ => "DEFAULT",
                };
                self.extra_options
                    .insert(opt.key.clone(), val_str.to_string());
            } else if opt.key == "musicToast" {
                let val_str = match opt.value.round() as i32 {
                    0 => "never",
                    1 => "pause",
                    2 => "pause_and_toast",
                    _ => "never",
                };
                let previous = self.extra_options.get("musicToast").map(String::as_str);
                let restart_toast = should_start_now_playing_toast_for_new_preference(
                    previous,
                    val_str,
                    self.now_playing_title.is_some(),
                    crate::client::sound_engine::SoundSourceVolumes::from_options(
                        &self.extra_options,
                    )
                    .final_source_volume(crate::client::sound_engine::SoundSource::Music),
                );
                self.extra_options
                    .insert(opt.key.clone(), val_str.to_string());
                if restart_toast {
                    self.now_playing_started_at = Some(web_time::Instant::now());
                }
            }
        }
    }
    fn flush_sound_option_changes(&mut self) {
        let (pending, selected_output) = match self.active_screen.as_mut() {
            Some(ActiveScreen::SoundOptions(screen)) => {
                let pending = screen.take_value_changed().then(|| screen.options.clone());
                let selected_output = screen
                    .take_audio_output_changed()
                    .then(|| screen.selected_audio_output_handle().map(str::to_owned))
                    .flatten();
                (pending, selected_output)
            }
            _ => (None, None),
        };
        if let Some(options) = pending {
            self.apply_sound_option_values(&options);
        }
        if let Some(handle) = selected_output {
            self.queue_audio_output_selection(handle);
        }
    }
    fn apply_world_options_settings(&mut self, screen: &el_gui::WorldOptionsScreen) {
        for opt in &screen.options {
            if opt.key == "allowCommands" {
                let bool_val = if opt.value > 0.5 { "true" } else { "false" };
                self.extra_options
                    .insert(opt.key.clone(), bool_val.to_string());
            } else if opt.key == "difficulty" {
                let val = opt.value.round() as i32;
                let (diff, val_str) = match val {
                    0 => (crate::world::client_level::Difficulty::Peaceful, "Peaceful"),
                    1 => (crate::world::client_level::Difficulty::Easy, "Easy"),
                    2 => (crate::world::client_level::Difficulty::Normal, "Normal"),
                    3 => (crate::world::client_level::Difficulty::Hard, "Hard"),
                    _ => (crate::world::client_level::Difficulty::Normal, "Normal"),
                };
                self.extra_options
                    .insert(opt.key.clone(), val_str.to_string());
                self.requested_difficulty = Some(diff);
            } else if opt.key == "gameMode" {
                let val = opt.value.round() as i32;
                let (mode, val_str) = match val {
                    0 => (GameType::Survival, "Survival"),
                    1 => (GameType::Creative, "Creative"),
                    2 => (GameType::Adventure, "Adventure"),
                    3 => (GameType::Spectator, "Spectator"),
                    _ => (GameType::Survival, "Survival"),
                };
                self.extra_options
                    .insert(opt.key.clone(), val_str.to_string());
                self.requested_game_mode = Some(mode);
            }
        }
        self.extra_options.insert(
            "difficultyLocked".to_string(),
            if screen.difficulty_locked {
                "true"
            } else {
                "false"
            }
            .to_string(),
        );
    }
    fn controls_screen(&self, in_world: bool) -> ControlsScreen {
        ControlsScreen::new_with_toggle_options(
            in_world,
            &self.extra_options,
            ControlsToggleOptions {
                toggle_crouch: self.toggle_crouch,
                toggle_sprint: self.toggle_sprint,
                toggle_attack: self.toggle_attack,
                toggle_use: self.toggle_use,
            },
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        )
    }
    fn apply_controls_options(&mut self, options: &[ControlsOption]) {
        for opt in options {
            if opt.key == "toggleCrouch" {
                self.toggle_crouch = opt.value > 0.5;
            } else if opt.key == "toggleSprint" {
                self.toggle_sprint = opt.value > 0.5;
            } else if opt.key == "toggleAttack" {
                self.toggle_attack = opt.value > 0.5;
            } else if opt.key == "toggleUse" {
                self.toggle_use = opt.value > 0.5;
            }
            self.extra_options
                .insert(opt.key.clone(), opt.option_text_value());
        }
    }
    fn apply_controls_settings(&mut self, screen: &el_gui::ControlsScreen) {
        self.apply_controls_options(&screen.options);
    }
    fn flush_controls_option_changes(&mut self) {
        let pending = match self.active_screen.as_mut() {
            Some(ActiveScreen::Controls(screen)) => {
                screen.take_value_changed().then(|| screen.options.clone())
            }
            _ => None,
        };
        if let Some(options) = pending {
            self.apply_controls_options(&options);
            self.save_options();
        }
    }
    fn apply_credits_and_attribution_settings(
        &mut self,
        screen: &el_gui::CreditsAndAttributionScreen,
    ) {
        for opt in &screen.options {
            self.extra_options
                .insert(opt.key.clone(), format!("{:.4}", opt.value));
        }
    }
    fn apply_accessibility_options_slice(&mut self, options: &[AccessibilityOptionsOption]) {
        for opt in options {
            if [
                "showSubtitles",
                "highContrast",
                "backgroundForChatOnly",
                "bobView",
                "hideLightningFlash",
                "darkMojangStudiosBackground",
                "hideSplashTexts",
                "narratorHotkey",
                "rotateWithMinecart",
                "highContrastBlockOutline",
            ]
            .contains(&opt.key.as_str())
            {
                let bool_val = if opt.value > 0.5 { "true" } else { "false" };
                self.extra_options
                    .insert(opt.key.clone(), bool_val.to_string());
                if opt.key == "bobView" {
                    self.view_bobbing = opt.value > 0.5;
                }
            } else if opt.key == "narrator" {
                let id = opt.value.round().clamp(0.0, 3.0) as i32;
                self.extra_options.insert(opt.key.clone(), id.to_string());
            } else {
                let norm = match opt.key.as_str() {
                    "notificationDisplayTime" => opt.value,
                    "chatDelay" => opt.value,
                    "menuBackgroundBlurriness" => opt.value,
                    "fovEffectScale" => opt.value,
                    _ => opt.value,
                };
                self.extra_options
                    .insert(opt.key.clone(), format!("{:.4}", norm));
            }
        }
    }
    fn apply_accessibility_options_settings(
        &mut self,
        screen: &el_gui::AccessibilityOptionsScreen,
    ) {
        self.apply_accessibility_options_slice(&screen.options);
    }
    pub fn open_inventory_screen(
        &mut self,
        player: &mut LocalPlayer,
        game_mode: GameType,
        inventory_state_id: i32,
        inventory_carried_item: el_core::item::ItemStack,
    ) {
        let mut player_inv = el_inventory::container::SimpleContainer::new_player_inventory(46);
        for i in 0..46.min(player.inventory.len()) {
            player_inv.set_item(i, player.inventory[i].clone());
        }
        let (menu, screen_type) = if game_mode == GameType::Creative {
            (
                el_inventory::menu::new_creative_menu(player_inv),
                el_inventory::menu::MenuType::Creative,
            )
        } else {
            (
                el_inventory::menu::new_inventory_menu(player_inv),
                el_inventory::menu::MenuType::Inventory,
            )
        };
        let title = if game_mode == GameType::Creative {
            crate::ui::i18n::translate("container.creative").to_string()
        } else {
            crate::ui::i18n::translate("container.crafting").to_string()
        };
        let mut screen = crate::ui::gui::screen::container::ContainerScreenState::new(
            0,
            screen_type,
            title,
            menu,
            inventory_state_id,
            inventory_carried_item,
        );
        screen.set_bundle_item_selection_connection_available(
            self.bundle_item_selection_connection_available,
        );
        let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
        let logical_w = self.last_screen_w / scale;
        let logical_h = self.last_screen_h / scale;
        screen.update_layout(logical_w, logical_h);
        self.active_screen = Some(ActiveScreen::Container(screen));
    }
    pub fn open_container(
        &mut self,
        container_id: i32,
        menu_type: el_inventory::menu::MenuType,
        title: String,
        player_inv: el_inventory::container::SimpleContainer,
        container_slots: usize,
    ) {
        let menu = match menu_type {
            el_inventory::menu::MenuType::Inventory => {
                el_inventory::menu::new_inventory_menu(player_inv)
            }
            el_inventory::menu::MenuType::Crafting => {
                el_inventory::menu::new_crafting_menu(player_inv)
            }
            el_inventory::menu::MenuType::Furnace => el_inventory::menu::new_furnace_menu(
                player_inv,
                el_inventory::container::SimpleContainer::new(3),
            ),
            el_inventory::menu::MenuType::Smoker => el_inventory::menu::new_furnace_menu(
                player_inv,
                el_inventory::container::SimpleContainer::new(3),
            ),
            el_inventory::menu::MenuType::BlastFurnace => el_inventory::menu::new_furnace_menu(
                player_inv,
                el_inventory::container::SimpleContainer::new(3),
            ),
            el_inventory::menu::MenuType::Chest => el_inventory::menu::new_chest_menu(
                player_inv,
                el_inventory::container::SimpleContainer::new(27),
            ),
            el_inventory::menu::MenuType::Chest1Row => el_inventory::menu::new_generic_menu(
                player_inv,
                9,
                el_inventory::menu::MenuType::Chest1Row,
            ),
            el_inventory::menu::MenuType::Chest2Rows => el_inventory::menu::new_generic_menu(
                player_inv,
                18,
                el_inventory::menu::MenuType::Chest2Rows,
            ),
            el_inventory::menu::MenuType::Chest3Rows => el_inventory::menu::new_generic_menu(
                player_inv,
                27,
                el_inventory::menu::MenuType::Chest3Rows,
            ),
            el_inventory::menu::MenuType::Mount => {
                el_inventory::menu::new_mount_menu(player_inv, container_slots)
            }
            el_inventory::menu::MenuType::Chest4Rows => el_inventory::menu::new_generic_menu(
                player_inv,
                36,
                el_inventory::menu::MenuType::Chest4Rows,
            ),
            el_inventory::menu::MenuType::Chest5Rows => el_inventory::menu::new_generic_menu(
                player_inv,
                45,
                el_inventory::menu::MenuType::Chest5Rows,
            ),
            el_inventory::menu::MenuType::Chest6Rows => el_inventory::menu::new_generic_menu(
                player_inv,
                54,
                el_inventory::menu::MenuType::Chest6Rows,
            ),
            el_inventory::menu::MenuType::Dispenser => {
                el_inventory::menu::new_dispenser_menu(player_inv)
            }
            el_inventory::menu::MenuType::Crafter => {
                el_inventory::menu::new_crafter_menu(player_inv)
            }
            el_inventory::menu::MenuType::Anvil => el_inventory::menu::new_anvil_menu(player_inv),
            el_inventory::menu::MenuType::Beacon => el_inventory::menu::new_beacon_menu(player_inv),
            el_inventory::menu::MenuType::BrewingStand => {
                el_inventory::menu::new_brewing_stand_menu(player_inv)
            }
            el_inventory::menu::MenuType::Hopper => el_inventory::menu::new_hopper_menu(player_inv),
            el_inventory::menu::MenuType::Lectern => {
                el_inventory::menu::new_lectern_menu(player_inv)
            }
            el_inventory::menu::MenuType::Loom => el_inventory::menu::new_loom_menu(player_inv),
            el_inventory::menu::MenuType::Merchant => {
                el_inventory::menu::new_merchant_menu(player_inv)
            }
            el_inventory::menu::MenuType::ShulkerBox => {
                el_inventory::menu::new_shulker_box_menu(player_inv)
            }
            el_inventory::menu::MenuType::Smithing => {
                el_inventory::menu::new_smithing_menu(player_inv)
            }
            el_inventory::menu::MenuType::Grindstone => {
                el_inventory::menu::new_grindstone_menu(player_inv)
            }
            el_inventory::menu::MenuType::CartographyTable => {
                el_inventory::menu::new_cartography_table_menu(player_inv)
            }
            el_inventory::menu::MenuType::Stonecutter => {
                el_inventory::menu::new_stonecutter_menu(player_inv)
            }
            el_inventory::menu::MenuType::Enchantment => {
                el_inventory::menu::new_enchantment_menu(player_inv)
            }
            el_inventory::menu::MenuType::Creative => {
                el_inventory::menu::new_creative_menu(player_inv)
            }
        };
        let mut screen = crate::ui::gui::screen::container::ContainerScreenState::new(
            container_id,
            menu_type,
            title,
            menu,
            0,
            el_core::item::ItemStack::empty(),
        );
        screen.set_bundle_item_selection_connection_available(
            self.bundle_item_selection_connection_available,
        );
        let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
        let logical_w = self.last_screen_w / scale;
        let logical_h = self.last_screen_h / scale;
        screen.update_layout(logical_w, logical_h);
        self.active_screen = Some(ActiveScreen::Container(screen));
    }
    pub fn set_container_data(&mut self, container_id: i32, id: usize, value: i32) {
        if let Some(ActiveScreen::Container(ref mut screen)) = self.active_screen {
            if screen.container_id == container_id {
                screen.set_data(id, value);
            }
        }
    }
    pub fn configure_title_screen(
        &mut self,
        version_name: impl Into<String>,
        demo: bool,
        multiplayer_allowed: bool,
    ) {
        self.title_version_name = version_name.into();
        self.title_demo = demo;
        self.title_multiplayer_allowed = multiplayer_allowed;
    }
    pub fn configure_player_profile(
        &mut self,
        username: impl Into<String>,
        profile_id: uuid::Uuid,
    ) {
        let username = username.into();
        if !username.trim().is_empty() {
            self.player_username = username;
        }
        self.player_profile_id = Some(profile_id);
    }
    pub(crate) fn player_username(&self) -> &str {
        &self.player_username
    }
    pub(crate) fn world_player_display_name(
        &self,
        uuid: Option<uuid::Uuid>,
        fallback_name: &str,
    ) -> String {
        let profile_name = uuid
            .and_then(|id| self.player_infos.get(&id))
            .map(|info| info.name.as_str())
            .filter(|name| !name.is_empty());
        world_player_display_name_from_profile(&self.scoreboard, profile_name, fallback_name)
    }
    pub fn configure_protocol_backend(&mut self, protocol_backend: ProtocolBackendData) {
        self.protocol_backend = protocol_backend;
    }
    pub fn game_directory(&self) -> std::path::PathBuf {
        self.options_path
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    }
    #[cfg(target_arch = "wasm32")]
    fn start_pinging_servers(&mut self, servers: &[el_gui::screens::multiplayer::ServerEntry]) {
        use wasm_bindgen::JsCast;
        let (tx, rx) = crossbeam_channel::unbounded();
        self.ping_results_rx = Some(rx);
        for (idx, entry) in servers.iter().enumerate() {
            let mut initial_entry = entry.clone();
            initial_entry.ping_state = el_gui::screens::multiplayer::ServerPingState::Pinging;
            let _ = tx.send((idx, initial_entry));
            let entry = entry.clone();
            let tx = tx.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let start_time = web_time::Instant::now();
                let mut updated_entry = entry.clone();
                let raw_ip = updated_entry.ip.trim();
                let clean_ip = if let Some(stripped) = raw_ip.strip_prefix("http://") {
                    stripped
                } else if let Some(stripped) = raw_ip.strip_prefix("https://") {
                    stripped
                } else {
                    raw_ip
                };
                let host_only = clean_ip
                    .strip_prefix('[')
                    .and_then(|address| address.split_once(']').map(|(host, _)| host))
                    .unwrap_or_else(|| clean_ip.split(':').next().unwrap_or(clean_ip));
                let loopback = host_only.eq_ignore_ascii_case("localhost")
                    || host_only
                        .parse::<std::net::IpAddr>()
                        .is_ok_and(|address| address.is_loopback());
                let mut fetch_success = false;
                if !loopback && let Some(window) = web_sys::window() {
                    let api_url = format!("https://api.mcsrvstat.us/3/{clean_ip}");
                    let opts = web_sys::RequestInit::new();
                    opts.set_method("GET");
                    opts.set_mode(web_sys::RequestMode::Cors);
                    if let Ok(request) = web_sys::Request::new_with_str_and_init(&api_url, &opts) {
                        let resp_promise = window.fetch_with_request(&request);
                        if let Ok(resp_val) =
                            wasm_bindgen_futures::JsFuture::from(resp_promise).await
                        {
                            if let Ok(resp) = resp_val.dyn_into::<web_sys::Response>() {
                                if resp.ok() {
                                    if let Ok(text_promise) = resp.text() {
                                        if let Ok(text_val) =
                                            wasm_bindgen_futures::JsFuture::from(text_promise).await
                                        {
                                            if let Some(text) = text_val.as_string() {
                                                if let Ok(val) =
                                                    serde_json::from_str::<serde_json::Value>(&text)
                                                {
                                                    let is_online =
                                                        val["online"].as_bool().unwrap_or(false);
                                                    if is_online {
                                                        fetch_success = true;
                                                        updated_entry.ping_state = el_gui::screens::multiplayer::ServerPingState::Successful;
                                                        updated_entry.status_display_state = el_gui::screens::multiplayer::ServerPingState::Successful;
                                                        updated_entry.status_text_ready = true;
                                                        updated_entry.ping =
                                                            start_time.elapsed().as_millis() as i64;
                                                        updated_entry.players_online =
                                                            val["players"]["online"]
                                                                .as_i64()
                                                                .unwrap_or(0)
                                                                as i32;
                                                        updated_entry.players_max =
                                                            val["players"]["max"]
                                                                .as_i64()
                                                                .unwrap_or(0)
                                                                as i32;
                                                        if let Some(ver_str) =
                                                            val["version"].as_str()
                                                        {
                                                            updated_entry.version =
                                                                ver_str.to_string();
                                                        } else if let Some(ver_name) =
                                                            val["protocol"]["name"].as_str()
                                                        {
                                                            updated_entry.version =
                                                                ver_name.to_string();
                                                        }
                                                        if let Some(clean_motd) =
                                                            val["motd"]["clean"].as_array()
                                                        {
                                                            let lines: Vec<String> = clean_motd
                                                                .iter()
                                                                .filter_map(|line| {
                                                                    line.as_str().map(|s| {
                                                                        s.trim().to_string()
                                                                    })
                                                                })
                                                                .collect();
                                                            updated_entry.motd = lines.join("\n");
                                                        } else if let Some(raw_motd) =
                                                            val["motd"]["raw"].as_array()
                                                        {
                                                            let lines: Vec<String> = raw_motd
                                                                .iter()
                                                                .filter_map(|line| {
                                                                    line.as_str().map(|s| {
                                                                        s.trim().to_string()
                                                                    })
                                                                })
                                                                .collect();
                                                            updated_entry.motd = lines.join("\n");
                                                        }
                                                        if let Some(icon_data) =
                                                            val["icon"].as_str()
                                                        {
                                                            if let Some(comma_pos) =
                                                                icon_data.find(',')
                                                            {
                                                                let b64 =
                                                                    &icon_data[comma_pos + 1..];
                                                                use base64::{
                                                                    Engine as _,
                                                                    prelude::BASE64_STANDARD,
                                                                };
                                                                if let Ok(bytes) =
                                                                    BASE64_STANDARD.decode(b64)
                                                                {
                                                                    updated_entry.icon_bytes =
                                                                        Some(bytes);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if !fetch_success {
                    updated_entry.ping_state =
                        el_gui::screens::multiplayer::ServerPingState::Unreachable;
                    updated_entry.status_display_state =
                        el_gui::screens::multiplayer::ServerPingState::Unreachable;
                    updated_entry.motd =
                        crate::ui::i18n::translate("multiplayer.status.cannot_connect").to_string();
                }
                let _ = tx.send((idx, updated_entry));
            });
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn ping_single_server(&mut self, entry: el_gui::screens::multiplayer::ServerEntry) {
        let tx = match self.ping_results_tx.as_ref() {
            Some(tx) if self.ping_results_rx.is_some() => tx.clone(),
            _ => {
                let (tx, rx) = crossbeam_channel::unbounded();
                self.ping_results_tx = Some(tx.clone());
                self.ping_results_rx = Some(rx);
                tx
            }
        };
        Self::spawn_ping_thread(tx, 0, entry, self.protocol_backend.wire_protocol_number());
    }
    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_ping_thread(
        tx: crossbeam_channel::Sender<(usize, el_gui::screens::multiplayer::ServerEntry)>,
        idx: usize,
        entry: el_gui::screens::multiplayer::ServerEntry,
        wire_protocol: Option<i32>,
    ) {
        std::thread::spawn(move || {
            use crate::network::multiplayer::resolver::server_address::ServerAddress;
            use crate::network::multiplayer::resolver::server_name_resolver::ServerNameResolver;
            use crate::network::multiplayer::server_pinger::{
                PingQueryError, query_legacy_server, query_minecraft_server,
            };
            use el_gui::screens::multiplayer::ServerPingState;
            let mut updated_entry = entry.clone();
            let Some(wire_protocol) = wire_protocol else {
                updated_entry.ping_state = ServerPingState::Unreachable;
                updated_entry.status_display_state = ServerPingState::Unreachable;
                updated_entry.motd = "Protocol backend is missing or incompatible".to_owned();
                updated_entry.motd_red = true;
                let _ = tx.send((idx, updated_entry));
                return;
            };
            let start = web_time::Instant::now();
            let raw_address = ServerAddress::parse_string(&updated_entry.ip);
            let resolved = ServerNameResolver::default().resolve_address(&raw_address);
            let Some(resolved) = resolved else {
                updated_entry.motd =
                    crate::ui::i18n::translate("multiplayer.status.cannot_connect").to_string();
                updated_entry.motd_red = true;
                updated_entry.player_list = Vec::new();
                let _ = tx.send((idx, updated_entry));
                return;
            };
            let sock_addr = resolved.as_socket_addr();
            let Ok(stream) =
                std::net::TcpStream::connect_timeout(&sock_addr, std::time::Duration::from_secs(3))
            else {
                updated_entry.ping_state = ServerPingState::Unreachable;
                updated_entry.status_display_state = ServerPingState::Unreachable;
                updated_entry.motd =
                    crate::ui::i18n::translate("multiplayer.status.cannot_connect").to_string();
                updated_entry.motd_red = true;
                updated_entry.player_list = Vec::new();
                let _ = tx.send((idx, updated_entry));
                return;
            };
            updated_entry.motd =
                crate::ui::i18n::translate("multiplayer.status.pinging").to_string();
            updated_entry.motd_red = false;
            let _ = tx.send((idx, updated_entry.clone()));
            let mut pong_state = ServerPingState::Incompatible;
            match query_minecraft_server(
                stream,
                raw_address.host(),
                raw_address.port(),
                wire_protocol,
                |status| {
                    let (version, protocol) = match status.version.as_ref() {
                        Some((name, protocol)) => (name.clone(), *protocol),
                        None => (
                            crate::ui::i18n::translate("multiplayer.status.old").to_string(),
                            0,
                        ),
                    };
                    pong_state = if protocol == wire_protocol {
                        ServerPingState::Successful
                    } else {
                        ServerPingState::Incompatible
                    };
                    updated_entry.version = version;
                    updated_entry.motd = status.motd.clone();
                    updated_entry.motd_runs = status.motd_runs.clone();
                    updated_entry.motd_red = false;
                    updated_entry.status_text_ready = true;
                    match status.players {
                        Some((online, max)) => {
                            updated_entry.players_present = true;
                            updated_entry.players_online = online;
                            updated_entry.players_max = max;
                        }
                        None => {
                            updated_entry.players_present = false;
                            updated_entry.players_online = 0;
                            updated_entry.players_max = 0;
                        }
                    }
                    updated_entry.player_list = status.player_list.clone();
                    if let Some(icon_bytes) = status.icon_bytes.clone() {
                        updated_entry.icon_bytes = Some(icon_bytes);
                    }
                    let _ = tx.send((idx, updated_entry.clone()));
                },
            ) {
                Ok(status) => {
                    let Some(ping) = status.ping else {
                        return;
                    };
                    updated_entry.ping_state = pong_state;
                    updated_entry.status_display_state = updated_entry.ping_state;
                    updated_entry.ping = ping;
                }
                Err(PingQueryError::Protocol) => {
                    updated_entry.motd =
                        crate::ui::i18n::translate("multiplayer.status.cannot_connect").to_string();
                    updated_entry.motd_red = true;
                    updated_entry.player_list = Vec::new();
                    if let Ok(legacy) = query_legacy_server(&sock_addr, raw_address.host()) {
                        updated_entry.ping_state = ServerPingState::Incompatible;
                        updated_entry.version = legacy.game_version;
                        updated_entry.motd = legacy.motd;
                        updated_entry.motd_red = false;
                        updated_entry.players_present = true;
                        updated_entry.players_online = legacy.current_players;
                        updated_entry.players_max = legacy.max_players;
                        updated_entry.player_list = Vec::new();
                        updated_entry.ping = start.elapsed().as_millis() as i64;
                    }
                }
            }
            let _ = tx.send((idx, updated_entry));
        });
    }
    #[cfg(target_arch = "wasm32")]
    fn start_lan_server_detector(&mut self) {}
    #[cfg(not(target_arch = "wasm32"))]
    fn start_lan_server_detector(&mut self) {
        self.stop_lan_server_detector();
        let (tx, rx) = crossbeam_channel::unbounded();
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let stop_thread = stop.clone();
        let detector = std::thread::Builder::new()
            .name("LanServerDetector".to_string())
            .spawn(move || {
                use crate::network::multiplayer::lan_server_pinger::{
                    MULTICAST_GROUP, PING_PORT, parse_address, parse_motd,
                };
                use std::net::{Ipv4Addr, UdpSocket};
                use std::time::Duration;
                let Ok(socket) = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, PING_PORT)) else {
                    return;
                };
                let Ok(group) = MULTICAST_GROUP.parse::<Ipv4Addr>() else {
                    return;
                };
                if socket
                    .join_multicast_v4(&group, &Ipv4Addr::UNSPECIFIED)
                    .is_err()
                {
                    return;
                }
                let _ = socket.set_read_timeout(Some(Duration::from_secs(5)));
                let mut servers: Vec<LanServerEntry> = Vec::new();
                let mut packet = [0_u8; 1024];
                while !stop_thread.load(std::sync::atomic::Ordering::Relaxed) {
                    match socket.recv_from(&mut packet) {
                        Ok((length, source)) => {
                            let text = String::from_utf8_lossy(&packet[..length]);
                            let motd = parse_motd(&text);
                            let Some(advertised) = parse_address(&text) else {
                                continue;
                            };
                            let address = format!("{}:{}", source.ip(), advertised);
                            if !servers.iter().any(|s| s.address == address) {
                                servers.push(LanServerEntry { motd, address });
                                let _ = tx.send(servers.clone());
                            }
                        }
                        Err(error)
                            if error.kind() == std::io::ErrorKind::WouldBlock
                                || error.kind() == std::io::ErrorKind::TimedOut => {}
                        Err(_) => break,
                    }
                }
                let _ = socket.leave_multicast_v4(&group, &Ipv4Addr::UNSPECIFIED);
            });
        if detector.is_ok() {
            self.lan_servers_rx = Some(rx);
            self.lan_detector_stop = Some(stop);
        }
    }
    fn stop_lan_server_detector(&mut self) {
        if let Some(stop) = self.lan_detector_stop.take() {
            stop.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.lan_servers_rx = None;
    }
    pub fn open_multiplayer_screen(&mut self, in_world: bool) {
        let skip_warning = self.option_bool("skipMultiplayerWarning", false);
        ui_crash_trace(format!(
            "Multiplayer stage=open in_world={in_world} warning_skipped={skip_warning}"
        ));
        if !skip_warning {
            self.active_screen = Some(ActiveScreen::Safety(SafetyScreen::new(
                in_world,
                std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            )));
            ui_crash_trace("Multiplayer stage=safety_screen_installed");
            return;
        }
        self.enter_multiplayer_screen(in_world);
    }
    pub fn enter_multiplayer_screen(&mut self, in_world: bool) {
        use crate::network::multiplayer::ServerList;
        ui_crash_trace(format!("Multiplayer stage=enter_begin in_world={in_world}"));
        let mut list = ServerList::new(self.game_directory());
        list.load();
        ui_crash_trace(format!(
            "Multiplayer stage=server_list_loaded servers={}",
            list.servers.len()
        ));
        let mut removed_invalid_icon = false;
        for server in &mut list.servers {
            if server
                .icon_bytes
                .as_deref()
                .is_some_and(|bytes| !is_valid_server_favicon(bytes))
            {
                server.icon_bytes = None;
                removed_invalid_icon = true;
            }
        }
        if removed_invalid_icon {
            list.save();
        }
        ui_crash_trace(format!(
            "Multiplayer stage=server_icons_checked invalid_removed={removed_invalid_icon}"
        ));
        let entries = Self::server_entries_from_list(&list);
        ui_crash_trace(format!(
            "Multiplayer stage=entries_built online={}",
            entries.len()
        ));
        let mut multiplayer = MultiplayerScreen::new(
            entries,
            in_world,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        if self.last_gui_input_type.is_keyboard() {
            multiplayer.apply_initial_keyboard_focus();
        }
        ui_crash_trace("Multiplayer stage=screen_constructed");
        self.start_lan_server_detector();
        ui_crash_trace("Multiplayer stage=lan_detector_started");
        let icon_dir = self.game_directory().join("server_icons");
        let _ = std::fs::create_dir_all(&icon_dir);
        let mut icon_mappings = Vec::new();
        for server in &list.servers {
            let Some(bytes) = server.icon_bytes.as_ref() else {
                continue;
            };
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            server.ip.hash(&mut hasher);
            bytes.hash(&mut hasher);
            let key = hasher.finish();
            let path = icon_dir.join(format!("{key:016x}.png"));
            if std::fs::write(&path, bytes).is_err() {
                continue;
            }
            let texture = ResourceLocation::new("eltharn", &format!("server_icons/{key:016x}"));
            multiplayer.set_server_icon(server.ip.clone(), texture.clone());
            icon_mappings.push((texture, path));
        }
        self.resource_manager = self
            .resource_manager
            .clone()
            .with_external_images(icon_mappings);
        self.stashed_multiplayer_screen = None;
        self.active_screen = Some(ActiveScreen::Multiplayer(multiplayer));
        ui_crash_trace("Multiplayer stage=screen_installed");
    }
    fn restore_multiplayer_screen(&mut self, in_world: bool) {
        if let Some(screen) = self.stashed_multiplayer_screen.take() {
            self.active_screen = Some(ActiveScreen::Multiplayer(screen));
        } else {
            self.enter_multiplayer_screen(in_world);
        }
    }
    fn server_entries_from_list(
        list: &crate::network::multiplayer::ServerList,
    ) -> Vec<ServerEntry> {
        use el_gui::screens::multiplayer::ServerPingState;
        list.servers
            .iter()
            .map(|s| ServerEntry {
                name: s.name.clone(),
                ip: s.ip.clone(),
                motd: String::new(),
                motd_red: false,
                motd_runs: Vec::new(),
                players_online: 0,
                players_max: 0,
                players_present: true,
                status_text_ready: false,
                ping: 0,
                ping_state: ServerPingState::Initial,
                status_display_state: ServerPingState::Initial,
                cached_pinging_frame: None,
                version: String::new(),
                player_list: Vec::new(),
                icon_bytes: s.icon_bytes.clone(),
            })
            .collect()
    }
    fn direct_connect_server_name(&mut self, ip: &str) -> String {
        use crate::network::multiplayer::ServerList;
        let mut list = ServerList::new(self.game_directory());
        list.load();
        if let Some(server) = list.get_by_ip(ip) {
            return server.name.clone();
        }
        let name = el_gui::translate("selectServer.defaultName");
        let new_server = crate::network::multiplayer::server_data::ServerData::new(
            name.clone(),
            ip.to_string(),
            crate::network::multiplayer::server_data::ServerType::Other,
        );
        list.add(new_server, true);
        list.save();
        name
    }
    fn apply_manage_server_done(
        &mut self,
        in_world: bool,
        edit_index: Option<usize>,
        name: String,
        ip: String,
        pack_status: ResourcePackStatus,
    ) {
        use crate::network::multiplayer::ServerList;
        use el_gui::screens::multiplayer::ServerPingState;
        let mut list = ServerList::new(self.game_directory());
        list.load();
        let server_pack_status = match pack_status {
            ResourcePackStatus::Enabled => {
                crate::network::multiplayer::server_data::ServerPackStatus::Enabled
            }
            ResourcePackStatus::Disabled => {
                crate::network::multiplayer::server_data::ServerPackStatus::Disabled
            }
            ResourcePackStatus::Prompt => {
                crate::network::multiplayer::server_data::ServerPackStatus::Prompt
            }
        };
        let name = if name.is_empty() {
            el_gui::translate("selectServer.defaultName")
        } else {
            name
        };
        let mut stash = self.stashed_multiplayer_screen.take();
        if let Some(idx) = edit_index {
            let mut new_server = crate::network::multiplayer::server_data::ServerData::new(
                name.clone(),
                ip.clone(),
                crate::network::multiplayer::server_data::ServerType::Other,
            );
            new_server.pack_status = server_pack_status;
            if let Some(old_server) = list.get(idx) {
                new_server.accepted_code_of_conduct = old_server.accepted_code_of_conduct;
                new_server.icon_bytes = old_server.icon_bytes.clone();
            }
            list.replace(idx, new_server);
            list.save();
            if let Some(mp) = stash.as_mut() {
                if let Some(entry) = mp.servers.get_mut(idx) {
                    let previous_ip = entry.ip.clone();
                    entry.name = name.clone();
                    entry.ip = ip.clone();
                    if previous_ip != ip {
                        if let Some(texture) = mp.remove_server_icon(&previous_ip) {
                            mp.set_server_icon(ip.clone(), texture);
                        }
                    }
                }
            }
        } else {
            let mut new_server = crate::network::multiplayer::server_data::ServerData::new(
                name.clone(),
                ip.clone(),
                crate::network::multiplayer::server_data::ServerType::Other,
            );
            new_server.pack_status = server_pack_status;
            let unhidden = list.unhide(&ip).is_some();
            if unhidden {
                if let Some(existing) = list.servers.iter_mut().find(|s| s.ip == ip) {
                    existing.copy_name_icon_from(&new_server);
                }
                list.save();
            } else {
                list.add(new_server, false);
                list.save();
            }
            if let Some(mp) = stash.as_mut() {
                let live_by_ip: std::collections::HashMap<String, ServerEntry> = mp
                    .servers
                    .iter()
                    .map(|entry| (entry.ip.clone(), entry.clone()))
                    .collect();
                let mut new_entries = Vec::new();
                for server in list.servers.iter() {
                    if let Some(mut live) = live_by_ip.get(&server.ip).cloned() {
                        live.name = server.name.clone();
                        if unhidden && server.ip == ip {
                            live.icon_bytes = None;
                            mp.remove_server_icon(&server.ip);
                        }
                        new_entries.push(live);
                    } else {
                        let entry = ServerEntry {
                            name: server.name.clone(),
                            ip: server.ip.clone(),
                            motd: String::new(),
                            motd_red: false,
                            motd_runs: Vec::new(),
                            players_online: 0,
                            players_max: 0,
                            players_present: true,
                            status_text_ready: false,
                            ping: 0,
                            ping_state: ServerPingState::Initial,
                            status_display_state: ServerPingState::Initial,
                            cached_pinging_frame: None,
                            version: String::new(),
                            player_list: Vec::new(),
                            icon_bytes: server.icon_bytes.clone(),
                        };
                        new_entries.push(entry);
                    }
                }
                mp.servers = new_entries;
                mp.selected_index = None;
                mp.selected_lan_index = None;
                mp.lan_header_selected = false;
            }
        }
        if let Some(mp) = stash {
            self.active_screen = Some(ActiveScreen::Multiplayer(mp));
        } else {
            self.enter_multiplayer_screen(in_world);
        }
    }
    pub fn start_quick_play_multiplayer(&mut self, server_address: String) {
        use crate::network::multiplayer::ServerList;
        let mut servers = ServerList::new(self.game_directory());
        servers.load();
        let server = servers.get_or_add_hidden_other_server(&server_address, || {
            el_gui::translate("selectServer.defaultName")
        });
        self.start_server_connection_with_cookies_and_origin(
            false,
            server.name,
            server_address,
            std::collections::HashMap::new(),
            false,
            ServerConnectionOrigin::QuickPlayMultiplayer,
        );
    }
    fn start_server_connection(&mut self, in_world: bool, server_name: String, server_ip: String) {
        log::debug!(
            "start_server_connection server_name={:?} server_ip={:?}",
            server_name,
            server_ip
        );
        self.start_server_connection_with_cookies(
            in_world,
            server_name,
            server_ip,
            std::collections::HashMap::new(),
            false,
        );
    }
    fn start_server_connection_with_cookies(
        &mut self,
        in_world: bool,
        server_name: String,
        server_ip: String,
        server_cookies: std::collections::HashMap<String, Vec<u8>>,
        is_transfer: bool,
    ) {
        self.start_server_connection_with_cookies_and_origin(
            in_world,
            server_name,
            server_ip,
            server_cookies,
            is_transfer,
            ServerConnectionOrigin::Normal,
        );
    }
    fn start_server_connection_with_cookies_and_origin(
        &mut self,
        in_world: bool,
        server_name: String,
        server_ip: String,
        server_cookies: std::collections::HashMap<String, Vec<u8>>,
        is_transfer: bool,
        origin: ServerConnectionOrigin,
    ) {
        let mut screen = ConnectingScreen::new(
            in_world,
            server_name,
            server_ip.clone(),
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        let username = if self.player_username.trim().is_empty() {
            "Player".to_string()
        } else {
            self.player_username.clone()
        };
        let mut args = ClientConnectRequest::offline(server_ip.clone(), username.clone());
        args.uuid = serverbound_hello_profile_id(self.player_profile_id, &username);
        args.view_distance = self.client_information_view_distance();
        args.language = self.client_information_language().to_owned();
        args.main_hand_left = self.client_information_main_hand_left();
        args.allows_listing = self.client_information_allows_listing();
        args.model_customisation = self.client_information_model_customisation();
        args.server_cookies = server_cookies;
        let mut server_list = crate::network::multiplayer::ServerList::new(self.game_directory());
        server_list.load();
        if let Some(server) = server_list.get_by_ip(&server_ip) {
            args.server_pack_status = match server.pack_status {
                crate::network::multiplayer::server_data::ServerPackStatus::Enabled => {
                    el_protocol_backend::SERVER_PACK_STATUS_ENABLED
                }
                crate::network::multiplayer::server_data::ServerPackStatus::Disabled => {
                    el_protocol_backend::SERVER_PACK_STATUS_DISABLED
                }
                crate::network::multiplayer::server_data::ServerPackStatus::Prompt => {
                    el_protocol_backend::SERVER_PACK_STATUS_PROMPT
                }
            };
        }
        self.live_server_pack_status = Some(args.server_pack_status);
        let requested_protocol = self.protocol_backend.requested_version.clone();
        let backend_path = self.protocol_backend.backend_path.clone();
        log::info!(
            "Starting Eltharn IPC connection to {server_ip} as {username}; view_distance={}; main_hand={}; allows_listing={}; server_pack_status={}; requested_protocol={} backend={:?}",
            args.view_distance,
            if args.main_hand_left { "left" } else { "right" },
            args.allows_listing,
            args.server_pack_status,
            requested_protocol,
            backend_path
        );
        screen.set_status("connect.joining", None);
        if let Some(previous) = self.server_connection.take() {
            drop(previous);
        }
        self.server_connection = Some(protocol_backend::spawn_connection(
            args,
            &self.protocol_backend,
        ));
        self.server_connection_is_transfer = is_transfer;
        self.server_connection_origin = origin;
        self.connecting_server_ip = Some(server_ip);
        if let Some(ActiveScreen::Multiplayer(mp)) = self.active_screen.as_ref() {
            if self.stashed_multiplayer_screen.is_none() {
                self.stashed_multiplayer_screen = Some(mp.clone());
            }
        }
        self.active_screen = Some(ActiveScreen::Connecting(screen));
    }
    pub fn start_server_transfer(
        &mut self,
        host: String,
        port: u32,
        server_cookies: std::collections::HashMap<String, Vec<u8>>,
    ) {
        let sanitized: String = host
            .chars()
            .filter(|c| !c.is_control() && *c != '\0')
            .collect();
        let mut target_host = sanitized.trim().to_string();
        if target_host.is_empty() {
            if let Some(last_ip) = self.extra_options.get("lastMpIp") {
                let clean_last: String = last_ip
                    .chars()
                    .filter(|c| !c.is_control() && *c != '\0')
                    .collect();
                let host_part = clean_last.split(':').next().unwrap_or("").trim();
                if !host_part.is_empty() {
                    target_host = host_part.to_string();
                }
            }
        }
        if target_host.is_empty() {
            log::debug!("start_server_transfer rejected: empty host (port={})", port);
            self.restore_multiplayer_screen(false);
            return;
        }
        let target_port = if port == 0 { 25565 } else { port };
        let server_address = format!("{target_host}:{target_port}");
        log::debug!(
            "start_server_transfer raw_host={:?} target_host={:?} port={} address={}",
            host,
            target_host,
            target_port,
            server_address
        );
        self.start_server_connection_with_cookies(
            true,
            "Transfer".to_string(),
            server_address,
            server_cookies,
            true,
        );
    }
    pub fn open_title_screen(&mut self) {
        self.menu_parent_stack.clear();
        self.stop_lan_server_detector();
        if self.cached_title_splash.is_none() {
            self.cached_title_splash = Some(self.load_title_splash());
        }
        let splash = self.cached_title_splash.as_ref().and_then(|s| s.clone());
        let settings = TitleScreenSettings {
            fading: false,
            splash,
            hide_splash_texts: self.option_bool("hideSplashTexts", false),
            version_name: self.title_version_name.clone(),
            modified: true,
            demo: self.title_demo,
            demo_world_present: false,
            multiplayer_allowed: self.title_multiplayer_allowed,
            multiplayer_disabled_reason: (!self.title_multiplayer_allowed)
                .then(|| crate::ui::i18n::translate("title.multiplayer.disabled").to_string()),
            show_easter_egg_logo: false,
        };
        self.active_screen = Some(ActiveScreen::Title(TitleScreen::new(
            settings,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        )));
    }
    pub fn open_pause_screen(&mut self) {
        self.menu_parent_stack.clear();
        self.active_screen = Some(ActiveScreen::Pause(PauseMenuScreen::new(
            true,
            self.has_singleplayer_server,
            self.is_local_server,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        )));
    }
    pub fn force_screen_for_capture(
        &mut self,
        id: &str,
        in_world: bool,
        controls: &crate::input::VanillaKeyMappings,
    ) -> bool {
        match id {
            "pack_prompt" | "pack_prompt_optional" => {
                self.open_server_resource_pack_prompt(
                    in_world,
                    uuid::Uuid::nil(),
                    "https://example.test/resources.zip".into(),
                    String::new(),
                    id == "pack_prompt",
                    Some("Server resources include custom textures and sounds.".into()),
                );
            }
            "custom_pause" => {
                self.set_server_links(vec![("Website".into(), "https://example.test".into())]);
                self.set_pause_screen_dialog(
                    "minecraft:server_links".into(),
                    Some(serde_json::json!(
                        { "type" : "minecraft:server_links", "title" : { "translate"
                        : "menu.server_links.title" }, "external_title" : { "text" :
                        "Server Rules...", "color" : "gold", "bold" : true } }
                    )),
                    true,
                );
                self.active_screen = Some(ActiveScreen::Pause(PauseMenuScreen::new(
                    true,
                    false,
                    false,
                    Arc::clone(&self.font) as Arc<dyn el_gui::FontWidthMeasurer>,
                )));
            }
            "none" => self.active_screen = None,
            "title" => self.open_title_screen(),
            "pause" => {
                self.active_screen = Some(ActiveScreen::Pause(PauseMenuScreen::new(
                    true,
                    self.has_singleplayer_server,
                    self.is_local_server,
                    std::sync::Arc::clone(&self.font)
                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                )));
            }
            "options" => {
                self.active_screen = Some(ActiveScreen::Options(OptionsScreen::new(
                    in_world,
                    std::sync::Arc::clone(&self.font)
                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                )));
            }
            "video" => {
                self.active_screen =
                    Some(ActiveScreen::Video(self.video_settings_screen(in_world)));
            }
            "controls" => {
                self.active_screen = Some(ActiveScreen::Controls(self.controls_screen(in_world)));
            }
            "keybinds" => {
                self.active_screen = Some(ActiveScreen::KeyBinds {
                    screen: KeyBindsScreen::new(
                        in_world,
                        build_keybind_entries(controls),
                        std::sync::Arc::clone(&self.font)
                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                    ),
                    waiting_for_key: None,
                });
            }
            "multiplayer" => self.open_multiplayer_screen(in_world),
            "resourcepacks" => self.open_resource_packs_screen(in_world),
            "chat_hover_text" | "chat_hover_item" | "chat_hover_entity" => {
                use crate::ui::domain::{ChatClickSpan, ChatHoverAction};
                let hover = match id {
                    "chat_hover_item" => {
                        let mut bytes = Vec::new();
                        let components = el_nbt::Tag::Compound(vec![
                            (
                                "minecraft:custom_name".into(),
                                el_nbt::Tag::Compound(vec![
                                    ("text".into(), el_nbt::Tag::String("Hover sword".into())),
                                    ("color".into(), el_nbt::Tag::String("gold".into())),
                                ]),
                            ),
                            ("minecraft:damage".into(), el_nbt::Tag::Int(3)),
                        ]);
                        el_nbt::io::write_named_tag(&mut bytes, "", &components).unwrap();
                        ChatHoverAction::ShowItem {
                            id: "minecraft:diamond_sword".into(),
                            count: 1,
                            tag: Some(bytes),
                        }
                    }
                    "chat_hover_entity" => {
                        self.extra_options
                            .insert("advancedItemTooltips".into(), "true".into());
                        ChatHoverAction::ShowEntity {
                            id: "minecraft:player".into(),
                            uuid: "12345678-1234-1234-1234-123456789abc".into(),
                            name: Some("§6Hover player".into()),
                        }
                    }
                    _ => ChatHoverAction::ShowText {
                        value: "§6§lGold hover text§r\nSecond line with vanilla formatting.".into(),
                    },
                };
                self.open_chat_screen();
                self.chat_component
                    .add_server_system_message_with_click_spans(
                        "Hover this message",
                        vec![ChatClickSpan {
                            start: 0,
                            end: 18,
                            action: None,
                            hover: Some(hover),
                            insertion: None,
                        }],
                        self.gui_ticks,
                    );
                let scale =
                    self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
                let options = self.chat_render_options();
                let chat_scale = options.scale.clamp(0.01, 1.0);
                let bottom = ((self.last_screen_h / scale
                    - crate::ui::chat::chat_component::ChatComponent::BOTTOM_MARGIN)
                    / chat_scale)
                    .floor()
                    * chat_scale;
                let top_offset = (8.0 * (options.line_spacing + 1.0) - 4.0 * options.line_spacing)
                    .round()
                    * chat_scale;
                self.physical_mouse_x = (4.0 * chat_scale + 4.0) * scale;
                self.physical_mouse_y = (bottom - top_offset + 2.0 * chat_scale) * scale;
            }
            "chat" => {
                self.chat_input_display_pos = 0;
                self.active_screen = Some(ActiveScreen::Chat {
                    input: String::new(),
                });
                self.chat_component.add_client_system_message(
                    "Chat visual probe: abc ABC 123 — wrapped message",
                    self.gui_ticks,
                );
            }
            "language" => {
                self.active_screen = Some(ActiveScreen::LanguageSelect(
                    self.create_language_select_screen(in_world),
                ));
            }
            "font" => {
                self.active_screen = Some(ActiveScreen::FontSettings(FontSettingsScreen::new(
                    in_world,
                    self.font_options().uniform,
                    self.font_options().japanese_variants,
                )));
            }
            "sound" => self.open_sound_options(in_world),
            "chat_options" => {
                self.active_screen = Some(ActiveScreen::ChatOptions(ChatOptionsScreen::new(
                    in_world,
                    &self.extra_options,
                    std::sync::Arc::clone(&self.font)
                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                )));
            }
            "accessibility" => {
                self.active_screen = Some(ActiveScreen::AccessibilityOptions(
                    AccessibilityOptionsScreen::new(
                        in_world,
                        &self.extra_options,
                        std::sync::Arc::clone(&self.font)
                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                    ),
                ));
            }
            "mouse" => {
                self.active_screen = Some(ActiveScreen::MouseSettings(MouseSettingsScreen::new(
                    in_world,
                    &self.extra_options,
                    self.sensitivity_setting,
                    f64::from(self.mouse_wheel_sensitivity),
                    std::sync::Arc::clone(&self.font)
                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                )));
            }
            "skin" => {
                self.active_screen = Some(ActiveScreen::SkinCustomization(
                    SkinCustomizationScreen::new(
                        in_world,
                        &self.extra_options,
                        std::sync::Arc::clone(&self.font)
                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                    ),
                ));
            }
            "world_options" => {
                self.active_screen = Some(ActiveScreen::WorldOptions(
                    self.world_options_screen(in_world),
                ));
            }
            "credits" => {
                self.active_screen = Some(ActiveScreen::CreditsAndAttribution(
                    CreditsAndAttributionScreen::new(
                        in_world,
                        false,
                        std::sync::Arc::clone(&self.font)
                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                    ),
                ));
            }
            "credits_roll" | "end_poem" => {
                let poem = id == "end_poem";
                match self.create_win_screen(poem) {
                    Ok(screen) => {
                        self.active_screen = Some(ActiveScreen::WinScreen {
                            screen,
                            parent: None,
                        });
                    }
                    Err(error) => {
                        log::error!("[autoshot] couldn't create WinScreen: {error}");
                        return false;
                    }
                }
            }
            "inventory" => return false,
            "mods" | "shaderpacks" | "mod-settings" => {
                return self.force_extension_screen_for_capture(id, in_world);
            }
            _ => return false,
        }
        true
    }
    pub fn open_chat_screen(&mut self) {
        self.open_chat_screen_with_initial("");
    }
    pub fn open_chat_screen_with_initial(&mut self, initial: &str) {
        let method = if initial.starts_with('/') {
            crate::ui::chat::chat_component::ChatMethod::Command
        } else {
            crate::ui::chat::chat_component::ChatMethod::Message
        };
        let restored_draft = if self.option_bool("saveChatDrafts", false) {
            self.chat_component.draft_text_for_method(method)
        } else {
            None
        };
        let is_draft = restored_draft.is_some();
        let input =
            screen::truncate_utf16(&restored_draft.unwrap_or_else(|| initial.to_string()), 256);
        self.chat_cursor = input.chars().count();
        self.chat_input_display_pos = 0;
        self.chat_selection_anchor = None;
        self.chat_history_pos = self.chat_component.recent_chat().len();
        self.chat_history_buffer.clear();
        self.chat_suggestion_index = 0;
        self.chat_suggestion_offset = 0;
        self.chat_tab_cycles = false;
        self.chat_suggestions_allowed = false;
        self.chat_suggestions_popup_open = false;
        self.chat_is_draft = is_draft;
        self.chat_component.reset_chat_scroll();
        self.active_screen = Some(ActiveScreen::Chat { input });
        if self.connected_chat {
            let request_input = match self.active_screen.as_ref() {
                Some(ActiveScreen::Chat { input }) => Some(input.clone()),
                _ => None,
            };
            if let Some(input) = request_input {
                self.queue_chat_suggestion_request(&input);
            }
        }
    }
    pub fn open_in_bed_chat_screen(&mut self) {
        self.open_in_bed_chat_screen_with_initial("");
    }
    pub fn open_in_bed_chat_screen_with_initial(&mut self, initial: &str) {
        let method = if initial.starts_with('/') {
            crate::ui::chat::chat_component::ChatMethod::Command
        } else {
            crate::ui::chat::chat_component::ChatMethod::Message
        };
        let restored_draft = if self.option_bool("saveChatDrafts", false) {
            self.chat_component.draft_text_for_method(method)
        } else {
            None
        };
        let is_draft = restored_draft.is_some();
        let input =
            screen::truncate_utf16(&restored_draft.unwrap_or_else(|| initial.to_string()), 256);
        self.chat_cursor = input.chars().count();
        self.chat_input_display_pos = 0;
        self.chat_selection_anchor = None;
        self.chat_history_pos = self.chat_component.recent_chat().len();
        self.chat_history_buffer.clear();
        self.chat_suggestion_index = 0;
        self.chat_suggestion_offset = 0;
        self.chat_tab_cycles = false;
        self.chat_suggestions_allowed = false;
        self.chat_suggestions_popup_open = false;
        self.chat_is_draft = is_draft;
        self.chat_component.reset_chat_scroll();
        let in_bed_screen = InBedChatScreen::new(
            &input,
            is_draft,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        self.active_screen = Some(ActiveScreen::InBedChat(in_bed_screen));
        if self.connected_chat {
            let request_input = match self.active_screen.as_ref() {
                Some(ActiveScreen::InBedChat(screen)) => Some(screen.input.clone()),
                _ => None,
            };
            if let Some(input) = request_input {
                self.queue_chat_suggestion_request(&input);
            }
        }
    }
    pub fn on_player_woke_up(&mut self) {
        if let Some(ActiveScreen::InBedChat(screen)) = self.active_screen.take() {
            let text = screen.input.clone();
            if !self.chat_is_draft && !text.is_empty() {
                self.chat_component.discard_draft();
                self.open_chat_screen_with_initial(&text);
            } else {
                self.chat_component.reset_chat_scroll();
                self.chat_component.save_as_draft(text);
                self.active_screen = None;
            }
        }
    }
    pub fn create_language_select_screen(&self, in_world: bool) -> LanguageSelectScreen {
        let current_lang = self
            .extra_options
            .get("lang")
            .cloned()
            .unwrap_or_else(|| "en_us".to_string());
        let languages = self.resource_manager.extract_languages();
        let mut screen = LanguageSelectScreen::with_languages(
            in_world,
            current_lang,
            languages,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        );
        screen.set_clipboard(self.clipboard.clone());
        screen
    }
    fn open_resource_packs_screen(&mut self, in_world: bool) {
        let pack_dir = self.resource_pack_directory();
        let mut discovered = crate::resources::pack_loader::scan_resource_packs(&pack_dir);
        let icon_mappings =
            crate::resources::pack_loader::register_pack_icons(&pack_dir, &mut discovered);
        self.resource_manager = self
            .resource_manager
            .clone()
            .with_external_images(icon_mappings);
        let selected_ids_str = self
            .extra_options
            .get("resourcePacks")
            .cloned()
            .unwrap_or_default();
        let selected_ids =
            crate::resources::pack_loader::option_value_to_pack_ids(&selected_ids_str);
        let mut selected: Vec<el_gui::settings::resource_packs::PackEntry> = Vec::new();
        for id in selected_ids.iter().rev() {
            if let Some(pos) = discovered.iter().position(|p| &p.id == id) {
                selected.push(discovered.remove(pos));
            }
        }
        selected.push(el_gui::settings::resource_packs::PackEntry {
            id: "vanilla".to_string(),
            title: el_gui::translate("resourcePack.vanilla.name").to_string(),
            description: el_gui::translate("resourcePack.vanilla.description").to_string(),
            compatibility: el_gui::PackCompatibility::Compatible,
            source: el_gui::PackSource::BuiltIn,
            icon_path: "minecraft:misc/unknown_pack".to_string(),
            has_icon: false,
            fixed_position: true,
            required: true,
        });
        let available = discovered;
        self.active_screen = Some(ActiveScreen::ResourcePacks(
            el_gui::settings::resource_packs::ResourcePacksScreen::new(
                in_world,
                available,
                selected,
                std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            ),
        ));
    }
    pub fn set_connected_chat(&mut self, connected: bool) {
        let became_connected = !self.connected_chat && connected;
        if self.connected_chat && !connected {
            self.server_command_names.clear();
            self.server_command_root = 0;
            self.server_command_nodes.clear();
            self.online_player_names.clear();
            self.custom_chat_completions.clear();
            self.server_chat_suggestions.clear();
            self.server_suggestion_input = None;
            self.last_suggestion_request_input = None;
            self.pending_chat_suggestion_requests.clear();
            self.pending_unattended_chat_commands.clear();
            self.pending_custom_click_actions.clear();
        }
        self.connected_chat = connected;
        if became_connected {
            let request_input = match self.active_screen.as_ref() {
                Some(ActiveScreen::Chat { input }) => Some(input.clone()),
                _ => None,
            };
            if let Some(input) = request_input {
                self.queue_chat_suggestion_request(&input);
            }
        }
    }
    pub fn set_bundle_item_selection_connection_available(&mut self, available: bool) {
        self.bundle_item_selection_connection_available = available;
        if let Some(ActiveScreen::Container(screen)) = self.active_screen.as_mut() {
            screen.set_bundle_item_selection_connection_available(available);
        }
    }
    pub fn take_pending_chat_submissions(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_chat_submissions)
    }
    pub fn take_pending_unattended_chat_commands(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_unattended_chat_commands)
    }
    pub fn take_pending_custom_click_actions(&mut self) -> Vec<(String, Option<Vec<u8>>)> {
        std::mem::take(&mut self.pending_custom_click_actions)
    }
    pub fn take_pending_server_resource_pack_responses(
        &mut self,
    ) -> Vec<ServerResourcePackResponse> {
        std::mem::take(&mut self.pending_server_resource_pack_responses)
    }
    pub fn open_server_resource_pack_prompt(
        &mut self,
        in_world: bool,
        id: uuid::Uuid,
        url: String,
        hash: String,
        required: bool,
        prompt: Option<String>,
    ) {
        let screen = PackConfirmScreen::new(
            in_world,
            id,
            url,
            hash,
            required,
            prompt,
            Arc::clone(&self.font) as Arc<dyn el_gui::FontWidthMeasurer>,
        );
        self.open_pack_confirm_screen(screen);
    }
    pub fn server_resource_pack_status(&self, backend_status: u8) -> u8 {
        self.live_server_pack_status.unwrap_or(backend_status)
    }
    fn persist_server_pack_decision(&mut self, accepted: bool, required: bool) {
        self.live_server_pack_status = Some(server_pack_status_after_decision(accepted));
        if !accepted && required {
            return;
        }
        let Some(ip) = self.connecting_server_ip.as_ref() else {
            return;
        };
        let mut list = crate::network::multiplayer::ServerList::new(self.game_directory());
        list.load();
        let mut changed = false;
        for server in list
            .servers
            .iter_mut()
            .chain(list.hidden_servers.iter_mut())
        {
            if server.ip == *ip {
                server.pack_status = if accepted {
                    crate::network::multiplayer::server_data::ServerPackStatus::Enabled
                } else {
                    crate::network::multiplayer::server_data::ServerPackStatus::Disabled
                };
                changed = true;
            }
        }
        if changed {
            list.save();
        }
    }
    pub fn set_attribute_base_values(&mut self, values: (f64, f64)) {
        if let Some(ActiveScreen::Container(screen)) = self.active_screen.as_mut() {
            screen.attribute_base_values = Some(values);
        }
    }
    pub fn take_pending_chat_suggestion_requests(&mut self) -> Vec<(u32, String)> {
        std::mem::take(&mut self.pending_chat_suggestion_requests)
    }
    pub fn set_server_command_tree(
        &mut self,
        mut commands: Vec<String>,
        root_index: u32,
        nodes: Vec<crate::ui::domain::CommandNode>,
    ) {
        commands.sort_unstable();
        commands.dedup();
        self.server_command_names = commands;
        self.server_command_root = root_index;
        self.server_command_nodes = nodes;
    }
    pub fn set_online_player_names(&mut self, mut names: Vec<String>) {
        names.sort_unstable();
        names.dedup();
        self.online_player_names = names;
        self.chat_suggestion_index = 0;
        self.chat_suggestion_offset = 0;
    }
    pub fn set_tab_list_header_footer(&mut self, header: String, footer: String) {
        self.tab_list_header = header;
        self.tab_list_footer = footer;
    }
    pub fn scoreboard_mut(&mut self) -> &mut crate::ui::scoreboard::Scoreboard {
        &mut self.scoreboard
    }
    pub fn scoreboard(&self) -> &crate::ui::scoreboard::Scoreboard {
        &self.scoreboard
    }
    pub fn set_title(&mut self, title: String) {
        self.server_title = Some(title);
        self.title_time = self
            .title_fade_in_time
            .saturating_add(self.title_stay_time)
            .saturating_add(self.title_fade_out_time);
    }
    pub fn set_subtitle(&mut self, subtitle: String) {
        self.server_subtitle = Some(subtitle);
    }
    pub fn set_title_times(&mut self, fade_in: u32, stay: u32, fade_out: u32) {
        if fade_in != u32::MAX {
            self.title_fade_in_time = fade_in;
        }
        if stay != u32::MAX {
            self.title_stay_time = stay;
        }
        if fade_out != u32::MAX {
            self.title_fade_out_time = fade_out;
        }
        if self.title_time > 0 {
            self.title_time = self
                .title_fade_in_time
                .saturating_add(self.title_stay_time)
                .saturating_add(self.title_fade_out_time);
        }
    }
    pub fn clear_titles(&mut self, reset_times: bool) {
        self.server_title = None;
        self.server_subtitle = None;
        self.title_time = 0;
        if reset_times {
            self.title_fade_in_time = 10;
            self.title_stay_time = 70;
            self.title_fade_out_time = 20;
        }
    }
    pub fn set_show_player_list(&mut self, show: bool) {
        let show = show
            && !(self.is_local_server
                && self
                    .player_infos
                    .values()
                    .filter(|info| info.listed)
                    .count()
                    <= 1
                && self
                    .scoreboard
                    .get_display_objective(crate::ui::scoreboard::DISPLAY_SLOT_LIST)
                    .is_none());
        if self.show_player_list != show {
            self.tab_health_states.clear();
        }
        self.show_player_list = show;
    }
    pub fn set_tablist_level_players(&mut self, players: impl IntoIterator<Item = uuid::Uuid>) {
        self.tablist_level_players.clear();
        self.tablist_level_players.extend(players);
    }
    pub fn set_online_mode(&mut self, online: bool) {
        self.online_mode = online;
    }
    pub fn reset_scoreboard_state(&mut self) {
        self.scoreboard.clear();
        self.player_infos.clear();
        self.tablist_level_players.clear();
        self.tab_list_header.clear();
        self.tab_list_footer.clear();
        self.tab_health_states.clear();
        self.pending_pack_parent = None;
        self.live_server_pack_status = None;
        self.pause_screen_dialog = None;
        self.show_player_list = false;
        self.hud.clear_boss_events();
        self.clear_titles(true);
    }
    pub fn apply_player_info_update(&mut self, entries: Vec<PlayerInfoUpdateEntry>) {
        apply_player_info_update_entries(&mut self.player_infos, entries);
    }
    pub fn add_player_info(
        &mut self,
        uuid: uuid::Uuid,
        name: String,
        texture_url: Option<String>,
        model: Option<String>,
    ) {
        if let Some(info) = self.player_infos.get_mut(&uuid) {
            if info.name.is_empty() {
                info.name = name;
            }
            if texture_url.is_some() {
                info.texture_url = texture_url;
            }
            if model.is_some() {
                info.model = model;
            }
            return;
        }
        self.player_infos.insert(
            uuid,
            crate::ui::scoreboard::PlayerInfoState {
                uuid,
                name,
                texture_url,
                model,
                game_mode: GameType::Survival,
                latency: 0,
                display_name: None,
                listed: true,
                list_order: 0,
                show_hat: true,
            },
        );
    }
    pub fn remove_player_info(&mut self, uuid: &uuid::Uuid) {
        self.player_infos.remove(uuid);
    }
    pub fn player_infos(
        &self,
    ) -> &std::collections::HashMap<uuid::Uuid, crate::ui::scoreboard::PlayerInfoState> {
        &self.player_infos
    }
    pub fn player_info(
        &self,
        uuid: &uuid::Uuid,
    ) -> Option<&crate::ui::scoreboard::PlayerInfoState> {
        self.player_infos.get(uuid)
    }
    pub fn set_server_context(&mut self, has_singleplayer_server: bool, is_local_server: bool) {
        if self.has_singleplayer_server == has_singleplayer_server
            && self.is_local_server == is_local_server
        {
            return;
        }
        self.has_singleplayer_server = has_singleplayer_server;
        self.is_local_server = is_local_server;
        if let Some(ActiveScreen::Pause(screen)) = self.active_screen.as_ref() {
            let show_pause_menu = screen.show_pause_menu();
            self.active_screen = Some(ActiveScreen::Pause(PauseMenuScreen::new(
                show_pause_menu,
                has_singleplayer_server,
                is_local_server,
                std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            )));
        }
    }
    pub fn set_remote_world_difficulty(
        &mut self,
        difficulty: Option<(crate::world::client_level::Difficulty, bool)>,
    ) {
        self.remote_world_difficulty = difficulty;
    }
    pub(crate) fn hud_background_options(&self) -> scoreboard_hud::HudBackgroundOptions {
        scoreboard_hud::HudBackgroundOptions {
            background_for_chat_only: self.option_bool("backgroundForChatOnly", true),
            text_background_opacity: self.option_float("textBackgroundOpacity", 0.5),
        }
    }
    pub fn update_custom_chat_completions(
        &mut self,
        action: ChatCompletionAction,
        entries: Vec<String>,
    ) {
        match action {
            ChatCompletionAction::Add => {
                self.custom_chat_completions.extend(entries);
            }
            ChatCompletionAction::Remove => {
                self.custom_chat_completions
                    .retain(|entry| !entries.iter().any(|remove| remove == entry));
            }
            ChatCompletionAction::Set => {
                self.custom_chat_completions = entries;
            }
        }
        self.custom_chat_completions.sort_unstable();
        self.custom_chat_completions.dedup();
        self.chat_suggestion_index = 0;
    }
    pub fn set_server_chat_suggestions(
        &mut self,
        id: u32,
        range_start: usize,
        range_end: usize,
        suggestions: Vec<NetworkChatSuggestion>,
    ) {
        if id != self.latest_suggestion_request_id {
            return;
        }
        let input = match self.active_screen.as_ref() {
            Some(ActiveScreen::Chat { input }) => input.as_str(),
            Some(ActiveScreen::InBedChat(screen)) => screen.input.as_str(),
            _ => return,
        };
        let range_start = char_index_at_java_utf16(input, range_start);
        let range_end = char_index_at_java_utf16(input, range_end);
        self.server_chat_suggestions = suggestions
            .into_iter()
            .map(|suggestion| {
                let display = suggestion.text.clone();
                let mut result = crate::ui::chat::ChatSuggestion::new(
                    display,
                    suggestion.text,
                    range_start,
                    range_end,
                );
                result.tooltip = suggestion.tooltip;
                result
            })
            .collect();
        self.server_suggestion_input = Some(input.to_string());
        if self.chat_suggestions_popup_open
            && self.chat_suggestion_original_contents.as_deref() == Some(input)
        {
            let mut s = self.server_chat_suggestions.clone();
            sort_server_chat_suggestions(&mut s, input, self.chat_cursor);
            self.active_chat_suggestions = s;
        }
        self.chat_suggestion_index = 0;
    }
    fn queue_chat_suggestion_request(&mut self, input: &str) {
        self.server_chat_suggestions.clear();
        self.server_suggestion_input = None;
        if !self.chat_suggestions_allowed
            || !self.connected_chat
            || !is_server_chat_suggestion_input(input)
            || self.last_suggestion_request_input.as_deref() == Some(input)
        {
            return;
        }
        self.latest_suggestion_request_id = self.latest_suggestion_request_id.wrapping_add(1);
        self.last_suggestion_request_input = Some(input.to_string());
        self.pending_chat_suggestion_requests
            .push((self.latest_suggestion_request_id, input.to_string()));
    }
    fn chat_suggestions_for(&self, input: &str) -> Vec<crate::ui::chat::ChatSuggestion> {
        if !self.chat_suggestions_allowed {
            return Vec::new();
        }
        if self.server_suggestion_input.as_deref() == Some(input) {
            let mut suggestions = self.server_chat_suggestions.clone();
            sort_server_chat_suggestions(&mut suggestions, input, self.chat_cursor);
            suggestions
        } else if !input.starts_with('/') {
            let mut completions = self.online_player_names.clone();
            completions.extend(self.custom_chat_completions.iter().cloned());
            completions.sort_unstable();
            completions.dedup();
            crate::ui::chat::CommandSuggestions::build_custom_chat_completions(
                input,
                self.chat_cursor,
                &completions,
            )
        } else if !self.server_command_nodes.is_empty() {
            crate::ui::chat::command_suggestions::build_chat_suggestions_with_command_nodes(
                input,
                self.chat_cursor,
                self.server_command_root,
                &self.server_command_nodes,
            )
        } else if self.connected_chat {
            Vec::new()
        } else {
            crate::ui::chat::CommandSuggestions::build_with_server_commands(
                input,
                self.chat_cursor,
                &[],
            )
        }
    }
    pub fn add_server_system_chat(&mut self, message: impl Into<String>) {
        self.add_server_system_chat_inner(message.into(), Vec::new(), true);
    }
    pub fn add_server_system_chat_with_click_spans(
        &mut self,
        message: impl Into<String>,
        click_spans: Vec<crate::ui::domain::ChatClickSpan>,
    ) {
        self.add_server_system_chat_inner(message.into(), click_spans, false);
    }
    fn add_server_system_chat_inner(
        &mut self,
        message: String,
        click_spans: Vec<crate::ui::domain::ChatClickSpan>,
        content_is_json_component: bool,
    ) {
        if self.chat_visibility_allows_system() {
            self.chat_component
                .add_message(crate::ui::chat::chat_component::GuiMessage {
                    added_time: self.gui_ticks,
                    content: message,
                    content_is_json_component,
                    click_spans,
                    signature: None,
                    source: crate::ui::chat::chat_component::GuiMessageSource::SystemServer,
                    tag: Some(crate::ui::chat::chat_component::GuiMessageTag::system()),
                });
        }
    }
    pub fn add_client_system_chat(&mut self, message: impl Into<String>) {
        if self.chat_visibility_allows_system() {
            self.chat_component
                .add_message(crate::ui::chat::chat_component::GuiMessage {
                    added_time: self.gui_ticks,
                    content: message.into(),
                    content_is_json_component: false,
                    click_spans: Vec::new(),
                    signature: None,
                    source: crate::ui::chat::chat_component::GuiMessageSource::SystemClient,
                    tag: Some(
                        crate::ui::chat::chat_component::GuiMessageTag::system_single_player(),
                    ),
                });
        }
    }
    pub fn add_player_chat(
        &mut self,
        message: impl Into<String>,
        signature: Option<crate::ui::chat::signed_message::MessageSignature>,
        tag: Option<crate::ui::chat::chat_component::GuiMessageTag>,
    ) {
        self.add_player_chat_with_click_spans(message, signature, tag, Vec::new());
    }
    pub fn add_player_chat_with_click_spans(
        &mut self,
        message: impl Into<String>,
        signature: Option<crate::ui::chat::signed_message::MessageSignature>,
        tag: Option<crate::ui::chat::chat_component::GuiMessageTag>,
        click_spans: Vec<crate::ui::domain::ChatClickSpan>,
    ) {
        if self.chat_visibility_allows_player() {
            self.chat_component.queue_message(
                crate::ui::chat::chat_component::GuiMessage {
                    added_time: self.gui_ticks,
                    content: message.into(),
                    content_is_json_component: false,
                    click_spans,
                    signature,
                    source: crate::ui::chat::chat_component::GuiMessageSource::Player,
                    tag,
                },
                self.gui_ticks,
            );
        }
    }
    pub fn delete_chat_message(
        &mut self,
        signature: &crate::ui::chat::signed_message::MessageSignature,
    ) {
        self.chat_component
            .delete_message(signature, self.gui_ticks);
    }
    pub fn clear_chat_messages(&mut self, clear_history: bool) {
        self.hud.chat_messages.clear();
        let command_history = std::mem::take(&mut self.recent_chat);
        self.chat_component
            .clear_messages(clear_history, &command_history);
        self.recent_chat = command_history;
    }
    pub fn on_disconnected(&mut self) {
        self.reset_scoreboard_state();
        self.clear_chat_messages(true);
        self.set_server_links(Vec::new());
    }
    fn chat_visibility_allows_player(&self) -> bool {
        match self.extra_options.get("chatVisibility").map(String::as_str) {
            Some(s) if s.eq_ignore_ascii_case("0") || s.eq_ignore_ascii_case("full") => true,
            None => true,
            _ => false,
        }
    }
    fn chat_visibility_allows_system(&self) -> bool {
        match self.extra_options.get("chatVisibility").map(String::as_str) {
            Some(s) if s.eq_ignore_ascii_case("2") || s.eq_ignore_ascii_case("hidden") => false,
            _ => true,
        }
    }
    fn chat_restrictions_screen(&self, in_world: bool) -> RestrictionsScreen {
        match self.extra_options.get("chatVisibility").map(String::as_str) {
            Some("2") | Some("hidden") => RestrictionsScreen::new(
                in_world,
                vec![GuiChatRestriction {
                    translation_key: "chat_restriction.chat_and_commands_disabled_by_options"
                        .to_string(),
                    action: Some(GuiChatRestrictionAction::GoToChatSettings),
                }],
                false,
                false,
                false,
                false,
                std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            ),
            Some("1") | Some("system") => RestrictionsScreen::new(
                in_world,
                vec![GuiChatRestriction {
                    translation_key: "chat_restriction.chat_disabled_by_options".to_string(),
                    action: Some(GuiChatRestrictionAction::GoToChatSettings),
                }],
                false,
                true,
                true,
                false,
                std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            ),
            _ => RestrictionsScreen::new(
                in_world,
                Vec::new(),
                true,
                true,
                true,
                true,
                std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            ),
        }
    }
    fn chat_render_options(&self) -> crate::ui::chat::chat_component::ChatRenderOptions {
        crate::ui::chat::chat_component::ChatRenderOptions {
            opacity: self.option_float("chatOpacity", 1.0),
            background_opacity: self.option_float("textBackgroundOpacity", 0.5),
            scale: self.option_float("chatScale", 1.0),
            line_spacing: self.option_float("chatLineSpacing", 0.0),
            width: self.option_float("chatWidth", 1.0),
            height_focused: self.option_float("chatHeightFocused", 1.0),
            height_unfocused: self.option_float("chatHeightUnfocused", 0.4375),
            colors: self.option_bool("chatColors", true),
            show_player_messages: self.chat_visibility_allows_player(),
            show_server_messages: self.chat_visibility_allows_system(),
        }
    }
    pub fn close_screen(&mut self) {
        if self.is_level_loading_screen_active() {
            self.finish_level_loading_screen();
        } else {
            self.active_screen = None;
        }
    }
    pub fn return_to_multiplayer_if_connecting(&mut self) {
        let in_world = match self.active_screen.as_ref() {
            Some(ActiveScreen::Connecting(screen)) => Some(screen.in_world()),
            _ => None,
        };
        if let Some(in_world) = in_world {
            self.connecting_server_ip = None;
            self.restore_multiplayer_screen(in_world);
        }
    }
    pub fn update_connecting_status(&mut self, status_key: &str, detail: Option<String>) {
        if let Some(ActiveScreen::Connecting(screen)) = self.active_screen.as_mut() {
            screen.set_status(status_key, detail);
        }
    }
    pub fn show_disconnect_screen(&mut self, in_world: bool, title: String, reason: String) {
        self.active_screen = Some(ActiveScreen::Disconnected(DisconnectedScreen::new(
            in_world,
            title,
            reason,
            None,
            None,
            None,
            self.title_multiplayer_allowed,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        )));
    }
    pub fn close_connecting_screen(&mut self) {
        if matches!(self.active_screen, Some(ActiveScreen::Connecting(_))) {
            self.active_screen = None;
        }
        if matches!(
            self.code_of_conduct_parent,
            Some(ActiveScreen::Connecting(_))
        ) {
            self.code_of_conduct_parent = None;
        }
    }
    pub fn connecting_server_has_accepted_code_of_conduct(&self, text: &str) -> bool {
        let ip = self
            .connecting_server_ip
            .clone()
            .or_else(|| self.extra_options.get("lastMpIp").cloned());
        let Some(ip) = ip else {
            return false;
        };
        let mut list = crate::network::multiplayer::ServerList::new(self.game_directory());
        list.load();
        list.get_by_ip(&ip)
            .map(|server| server.has_accepted_code_of_conduct(text))
            .unwrap_or(false)
    }
    pub fn open_code_of_conduct_screen(&mut self, text: String, in_world: bool) {
        self.code_of_conduct_parent = self.active_screen.take();
        self.active_screen = Some(ActiveScreen::CodeOfConduct(CodeOfConductScreen::new(
            in_world,
            text,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        )));
    }
    fn world_options_screen(&self, in_world: bool) -> WorldOptionsScreen {
        let mut options = self.extra_options.clone();
        if !self.has_singleplayer_server {
            if let Some((difficulty, locked)) = self.remote_world_difficulty {
                let name = match difficulty {
                    crate::world::client_level::Difficulty::Peaceful => "Peaceful",
                    crate::world::client_level::Difficulty::Easy => "Easy",
                    crate::world::client_level::Difficulty::Normal => "Normal",
                    crate::world::client_level::Difficulty::Hard => "Hard",
                };
                options.insert("difficulty".to_string(), name.to_string());
                options.insert(
                    "difficultyLocked".to_string(),
                    if locked { "true" } else { "false" }.to_string(),
                );
            }
        }
        WorldOptionsScreen::new_with_server_context(
            in_world,
            &options,
            self.has_singleplayer_server,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        )
    }
    fn open_options_submenu(&mut self, submenu: PauseSubmenu, in_world: bool) {
        let font =
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>;
        match submenu {
            PauseSubmenu::Options => {
                self.active_screen =
                    Some(ActiveScreen::Options(OptionsScreen::new(in_world, font)));
            }
            PauseSubmenu::WorldOptions => {
                self.active_screen = Some(ActiveScreen::WorldOptions(
                    self.world_options_screen(in_world),
                ));
            }
            PauseSubmenu::SkinCustomization => {
                self.active_screen = Some(ActiveScreen::SkinCustomization(
                    SkinCustomizationScreen::new(
                        in_world,
                        &self.extra_options,
                        std::sync::Arc::clone(&self.font)
                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                    ),
                ));
            }
            PauseSubmenu::MusicAndSounds => {
                self.open_sound_options(in_world);
            }
            PauseSubmenu::OnlineOptions => {
                self.active_screen = Some(ActiveScreen::OnlineOptions(OnlineOptionsScreen::new(
                    in_world,
                    self.client_information_allows_listing(),
                    font,
                )));
            }
            PauseSubmenu::Telemetry => {
                self.active_screen = Some(ActiveScreen::TelemetryInfo(TelemetryInfoScreen::new(
                    in_world,
                    TelemetryInfoCapabilities::default(),
                    false,
                    font,
                )));
            }
            PauseSubmenu::VideoSettings => {
                self.active_screen =
                    Some(ActiveScreen::Video(self.video_settings_screen(in_world)));
            }
            PauseSubmenu::Controls => {
                self.active_screen = Some(ActiveScreen::Controls(self.controls_screen(in_world)));
            }
            PauseSubmenu::Language => {
                self.active_screen = Some(ActiveScreen::LanguageSelect(
                    self.create_language_select_screen(in_world),
                ));
            }
            PauseSubmenu::ChatSettings => {
                self.active_screen = Some(ActiveScreen::ChatOptions(ChatOptionsScreen::new(
                    in_world,
                    &self.extra_options,
                    std::sync::Arc::clone(&self.font)
                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                )));
            }
            PauseSubmenu::ResourcePacks => {
                self.open_resource_packs_screen(in_world);
            }
            PauseSubmenu::Accessibility => {
                self.active_screen = Some(ActiveScreen::AccessibilityOptions(
                    AccessibilityOptionsScreen::new(
                        in_world,
                        &self.extra_options,
                        std::sync::Arc::clone(&self.font)
                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                    ),
                ));
            }
            PauseSubmenu::CreditsAndAttribution => {
                self.active_screen = Some(ActiveScreen::CreditsAndAttribution(
                    CreditsAndAttributionScreen::new(in_world, false, font),
                ));
            }
            other => {
                self.active_screen = Some(ActiveScreen::SubmenuStub {
                    submenu: other,
                    in_world,
                });
            }
        }
    }
    fn open_sound_options(&mut self, in_world: bool) {
        let font =
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>;
        let mut screen = SoundOptionsScreen::new(in_world, &self.extra_options, font);
        screen.set_directional_audio_supported(
            crate::client::audio::AudioEngine::supports_directional_audio(),
        );
        if let Some(catalog) = self.audio_output_catalog.as_ref() {
            let choices =
                if catalog.supported {
                    let mut choices = Vec::with_capacity(catalog.devices.len() + 1);
                    choices.push(el_gui::SoundOutputChoice {
                        handle: String::new(),
                        name: crate::ui::i18n::translate("options.audioDevice.default").to_string(),
                    });
                    choices.extend(catalog.devices.iter().map(|device| {
                        el_gui::SoundOutputChoice {
                            handle: device.handle.clone(),
                            name: device.name.clone(),
                        }
                    }));
                    choices
                } else {
                    Vec::new()
                };
            screen.set_audio_output_choices(choices, &catalog.preferred);
        }
        self.active_screen = Some(ActiveScreen::SoundOptions(screen));
        self.pending_audio_output_refresh = true;
    }
    fn stash_current_menu_parent(&mut self, current: &mut ActiveScreen) {
        let mut placeholder = ActiveScreen::NavigationTransition;
        std::mem::swap(current, &mut placeholder);
        self.menu_parent_stack.push(placeholder);
    }
    fn menu_parent_or_options(&mut self, in_world: bool) -> ActiveScreen {
        self.pop_menu_parent_or(ActiveScreen::Options(OptionsScreen::new(
            in_world,
            std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
        )))
    }
    fn pop_menu_parent_or(&mut self, fallback: ActiveScreen) -> ActiveScreen {
        self.menu_parent_stack.pop().unwrap_or(fallback)
    }
    fn close_submenu_stub(&mut self, in_world: bool, is_options_submenu: bool) {
        if let Some(parent) = self.menu_parent_stack.pop() {
            self.active_screen = Some(parent);
        } else if is_options_submenu {
            self.active_screen = Some(self.menu_parent_or_options(in_world));
        } else if in_world {
            self.active_screen = Some(ActiveScreen::Pause(PauseMenuScreen::new(
                true,
                self.has_singleplayer_server,
                self.is_local_server,
                std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            )));
        } else {
            self.open_title_screen();
        }
    }
    pub fn open_confirm_screen(&mut self, screen: ConfirmScreen) {
        self.active_screen = Some(ActiveScreen::Confirm(screen));
    }
    pub fn open_pack_confirm_screen(&mut self, screen: PackConfirmScreen) {
        self.active_screen = Some(install_pack_prompt(
            self.active_screen.take(),
            &mut self.pending_pack_parent,
            screen,
        ));
    }
    pub fn open_confirm_link_screen(&mut self, screen: ConfirmLinkScreen) {
        self.active_screen = Some(ActiveScreen::ConfirmLink(screen));
    }
    fn return_from_link_prompt(&mut self) {
        match self.pending_link_return_input.take() {
            Some(input) => self.open_chat_screen_with_initial(&input),
            None => {
                self.active_screen = self.menu_parent_stack.pop();
            }
        }
    }
    fn apply_code_of_conduct_result(&mut self, accepted: bool, stop_showing: bool, text: &str) {
        use crate::network::multiplayer::ServerList;
        let ip = self
            .connecting_server_ip
            .clone()
            .or_else(|| self.extra_options.get("lastMpIp").cloned());
        let Some(ip) = ip else {
            return;
        };
        let mut list = ServerList::new(self.game_directory());
        list.load();
        let mut changed = false;
        for server in list
            .servers
            .iter_mut()
            .chain(list.hidden_servers.iter_mut())
        {
            if server.ip == ip {
                if accepted && stop_showing {
                    server.accept_code_of_conduct(text);
                } else {
                    server.clear_code_of_conduct();
                }
                changed = true;
                break;
            }
        }
        if changed {
            list.save();
        }
    }
    pub fn close_code_of_conduct_acknowledged(&mut self) {
        self.mark_screen_dirty();
        if let Some(parent) = self.code_of_conduct_parent.take() {
            self.active_screen = Some(parent);
        } else {
            self.active_screen = None;
        }
    }
    pub fn handle_input(
        &mut self,
        event: &WindowEvent,
        player: &mut LocalPlayer,
        _audio_engine: &mut crate::client::audio::AudioEngine,
        _resource_manager: &ClientResourceManager,
        controls: &mut crate::input::VanillaKeyMappings,
    ) -> bool {
        #[cfg(feature = "extensions")]
        crate::input::button_state::observe(event);
        self.mark_screen_dirty();
        if self.active_screen.is_some() {
            match event {
                WindowEvent::MouseInput { .. } => {
                    self.last_gui_input_type = GuiInputType::Mouse;
                }
                WindowEvent::KeyboardInput {
                    event: key_event, ..
                } => match key_event.physical_key {
                    winit::keyboard::PhysicalKey::Code(KeyCode::Tab) => {
                        self.last_gui_input_type = GuiInputType::KeyboardTab;
                    }
                    winit::keyboard::PhysicalKey::Code(
                        KeyCode::ArrowLeft
                        | KeyCode::ArrowRight
                        | KeyCode::ArrowUp
                        | KeyCode::ArrowDown,
                    ) => {
                        self.last_gui_input_type = GuiInputType::KeyboardArrow;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        let click_to_record = if let WindowEvent::MouseInput {
            button,
            state: ElementState::Pressed,
            ..
        } = event
        {
            let screen_id = self
                .active_screen
                .as_ref()
                .and_then(ActiveScreen::screen_instance_id);
            screen_id.map(|screen_id| {
                let button_id = gui_mouse_button_id(*button);
                let now = web_time::Instant::now();
                let double_click = is_java_double_click(
                    self.last_migrated_screen_click.as_ref(),
                    now,
                    screen_id,
                    button_id,
                );
                if let Some(screen) = self
                    .active_screen
                    .as_mut()
                    .and_then(ActiveScreen::migrated_screen_mut)
                {
                    screen.base().set_pending_double_click(double_click);
                }
                (now, screen_id, button_id)
            })
        } else {
            None
        };
        if let WindowEvent::ModifiersChanged(modifiers) = event {
            self.shift_held = modifiers.state().shift_key();
            self.control_held = modifiers.state().control_key();
            self.alt_held = modifiers.state().alt_key();
            for (category, held) in [self.shift_held, self.control_held, self.alt_held]
                .into_iter()
                .enumerate()
            {
                if !held {
                    self.held_modifier_sides &= !(3 << (category * 2));
                }
            }
        }
        if matches!(event, WindowEvent::Focused(false)) {
            self.shift_held = false;
            self.control_held = false;
            self.alt_held = false;
            self.held_modifier_sides = 0;
        }
        if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
        {
            if let winit::keyboard::PhysicalKey::Code(code) = key_event.physical_key {
                if let Some((category, held)) =
                    update_held_gui_modifier(&mut self.held_modifier_sides, code, key_event.state)
                {
                    match category {
                        0 => self.shift_held = held,
                        1 => self.control_held = held,
                        _ => self.alt_held = held,
                    }
                }
            }
        }
        if let WindowEvent::MouseInput { button, state, .. } = event {
            if *button == MouseButton::Left {
                self.mouse_down_left = *state == ElementState::Pressed;
            }
        }
        if let WindowEvent::CursorMoved { position, .. } = event {
            let old_mouse_x = self.mouse_x;
            let old_mouse_y = self.mouse_y;
            self.physical_mouse_x = position.x as f32;
            self.physical_mouse_y = position.y as f32;
            self.update_logical_mouse();
            let dx = self.mouse_x - old_mouse_x;
            let dy = self.mouse_y - old_mouse_y;
            if dx != 0.0 || dy != 0.0 {
                if let Some(screen) = self
                    .active_screen
                    .as_mut()
                    .and_then(ActiveScreen::migrated_screen_mut)
                {
                    el_gui::framework::screen::screen_mouse_moved(
                        screen,
                        self.mouse_x as f64,
                        self.mouse_y as f64,
                    );
                }
            }
            let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
            let logical_w = self.last_screen_w / scale;
            let logical_h = self.last_screen_h / scale;
            if let Some(ActiveScreen::Options(options_screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::mouse_drag_event;
                use el_gui::framework::screen::Screen;
                let modifiers = crate::ui::gui::screen_events::HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    options_screen.mouse_dragged(
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    if let Some(raw) = options_screen.take_fov_drag() {
                        let degrees = el_gui::settings::options::fov_degrees_from_slider(raw);
                        self.fov_setting =
                            el_gui::settings::options::fov_slider_from_degrees(degrees);
                    }
                    return true;
                }
            }
            if let Some(ActiveScreen::Video(video_screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    video_screen.mouse_dragged(
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    self.apply_active_video_settings_live();
                    return true;
                }
            }
            if let Some(ActiveScreen::SkinCustomization(screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    Screen::mouse_dragged(
                        screen,
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::ChatOptions(screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    screen.mouse_dragged(
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    self.apply_active_chat_options_live();
                    return true;
                }
            }
            if let Some(ActiveScreen::SoundOptions(screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    screen.mouse_dragged(
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    self.flush_sound_option_changes();
                    return true;
                }
            }
            if let Some(ActiveScreen::WorldOptions(screen)) = self.active_screen.as_mut() {
                screen.mouse_dragged(
                    self.mouse_x,
                    self.mouse_y,
                    dy,
                    logical_w.round() as i32,
                    logical_h.round() as i32,
                );
            }
            if let Some(ActiveScreen::Controls(screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    Screen::mouse_dragged(
                        screen,
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    self.flush_controls_option_changes();
                    return true;
                }
            }
            if let Some(ActiveScreen::CreditsAndAttribution(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let _ = screen.mouse_dragged(
                    el_gui::input::events::MouseButtonEvent::new(
                        self.mouse_x as f64,
                        self.mouse_y as f64,
                        el_gui::input::events::MouseButtonInfo::new(0, 0),
                    ),
                    dx as f64,
                    dy as f64,
                );
            }
            if let Some(ActiveScreen::Multiplayer(screen)) = self.active_screen.as_mut() {
                Screen::mouse_dragged(
                    screen,
                    el_gui::input::events::MouseButtonEvent::new(
                        self.mouse_x as f64,
                        self.mouse_y as f64,
                        el_gui::input::events::MouseButtonInfo::new(0, 0),
                    ),
                    dx as f64,
                    dy as f64,
                );
            }
            if let Some(ActiveScreen::Restrictions(screen)) = self.active_screen.as_mut() {
                screen.mouse_dragged(
                    self.mouse_x,
                    self.mouse_y,
                    dy,
                    logical_w.round() as i32,
                    logical_h.round() as i32,
                    &self.font,
                );
            }
            if let Some(ActiveScreen::AccessibilityOptions(screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    Screen::mouse_dragged(
                        screen,
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    self.apply_active_accessibility_options_live();
                    return true;
                }
            }
            if let Some(ActiveScreen::MouseSettings(screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    screen.mouse_dragged(
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::LanguageSelect(screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    Screen::mouse_dragged(
                        screen,
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::KeyBinds { screen, .. }) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    Screen::mouse_dragged(
                        screen,
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::Container(screen)) = self.active_screen.as_mut() {
                screen.handle_mouse_move(self.mouse_x, self.mouse_y);
            }
            if let Some(ActiveScreen::ResourcePacks(screen)) = self.active_screen.as_mut() {
                use crate::ui::gui::screen_events::{HeldModifiers, mouse_drag_event};
                use el_gui::framework::screen::Screen;
                let modifiers = HeldModifiers {
                    shift: self.shift_held,
                    control: self.control_held,
                };
                let handled = if self.mouse_down_left {
                    Screen::mouse_dragged(
                        screen,
                        mouse_drag_event(self.mouse_x as f64, self.mouse_y as f64, modifiers),
                        dx as f64,
                        dy as f64,
                    )
                } else {
                    false
                };
                if handled {
                    return true;
                }
            }
            let chat_input = match self.active_screen.as_ref() {
                Some(ActiveScreen::Chat { input }) => Some(input.as_str()),
                Some(ActiveScreen::InBedChat(screen)) => Some(screen.input.as_str()),
                _ => None,
            };
            if let Some(input) = chat_input {
                if self.mouse_down_left
                    && self.mouse_y >= logical_h - 14.0
                    && self.mouse_y < logical_h
                {
                    let input_width = (logical_w - 4.0).max(1.0);
                    let (display_start, display_end) = chat_input_display_range(
                        &self.font,
                        input,
                        self.chat_input_display_pos,
                        input_width,
                    );
                    let start_byte = byte_index_at_char(input, display_start);
                    let end_byte = byte_index_at_char(input, display_end);
                    let displayed_input = &input[start_byte..end_byte];
                    let clicked = display_start
                        + chat_cursor_from_x(
                            &self.font,
                            displayed_input,
                            (self.mouse_x - 4.0).max(0.0),
                        );
                    if self.chat_selection_anchor.is_none() {
                        self.chat_selection_anchor = Some(self.chat_cursor);
                    }
                    self.chat_cursor = clicked;
                }
            }
            self.apply_active_video_settings_live();
        }
        if let WindowEvent::MouseWheel { delta, .. } = event {
            let (raw_x, raw_y) = match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => (*x as f64, *y as f64),
                winit::event::MouseScrollDelta::PixelDelta(pos) => (pos.x / 12.0, pos.y / 12.0),
            };
            let sensitivity = self.mouse_wheel_sensitivity as f64;
            let scale_scroll = |raw: f64| {
                (if self.discrete_mouse_scroll {
                    if raw > 0.0 {
                        1.0
                    } else if raw < 0.0 {
                        -1.0
                    } else {
                        0.0
                    }
                } else {
                    raw
                }) * sensitivity
            };
            let scaled_x = scale_scroll(raw_x) as f32;
            let scaled_y = scale_scroll(raw_y) as f32;
            if matches!(
                self.active_screen,
                Some(ActiveScreen::Chat { .. }) | Some(ActiveScreen::InBedChat(_))
            ) {
                let chat_input = match self.active_screen.as_ref() {
                    Some(ActiveScreen::Chat { input }) => Some(input.as_str()),
                    Some(ActiveScreen::InBedChat(screen)) => Some(screen.input.as_str()),
                    _ => None,
                };
                if let Some(input) = chat_input {
                    let suggestions = self.chat_suggestions_for(input);
                    if !suggestions.is_empty() {
                        let limit = suggestions.len().min(10);
                        let width = suggestions
                            .iter()
                            .map(|suggestion| self.font.measure_width(&suggestion.display, 1.0))
                            .map(f32::ceil)
                            .fold(0.0_f32, f32::max);
                        let scale = self
                            .get_gui_scale_factor(self.last_screen_w, self.last_screen_h)
                            as f32;
                        let logical_w = self.last_screen_w / scale;
                        let logical_h = self.last_screen_h / scale;
                        let x = chat_suggestion_x(
                            &self.font,
                            input,
                            suggestions
                                .first()
                                .map(|suggestion| suggestion.range_start)
                                .unwrap_or(0),
                            self.chat_input_display_pos,
                            width,
                            logical_w,
                        );
                        let y = logical_h - 15.0 - limit as f32 * 12.0;
                        let inside = self.mouse_x > x
                            && self.mouse_x < x + width + 1.0
                            && self.mouse_y > y
                            && self.mouse_y < y + limit as f32 * 12.0;
                        if inside {
                            let max_offset = suggestions.len().saturating_sub(10);
                            let delta = scaled_y.clamp(-1.0, 1.0) as i32;
                            self.chat_suggestion_offset = if delta > 0 {
                                self.chat_suggestion_offset.saturating_sub(delta as usize)
                            } else {
                                self.chat_suggestion_offset
                                    .saturating_add(delta.unsigned_abs() as usize)
                            }
                            .min(max_offset);
                            return true;
                        }
                    }
                }
                let amount = scaled_y.clamp(-1.0, 1.0) * if self.shift_held { 1.0 } else { 7.0 };
                let options = self.chat_render_options();
                self.chat_component
                    .scroll_chat(amount as i32, &self.font, options, true);
                return true;
            }
            let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
            let logical_h = self.last_screen_h / scale;
            if let Some(ActiveScreen::Container(screen)) = self.active_screen.as_mut() {
                let _ = screen.mouse_scrolled(self.mouse_x, self.mouse_y, scaled_x, scaled_y);
                self.pending_bundle_item_selections
                    .extend(screen.take_pending_bundle_item_selections());
                self.pending_container_slot_state_changes
                    .extend(screen.take_pending_slot_state_changes());
                return true;
            }
            if let Some(ActiveScreen::Video(video_screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = video_screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    self.apply_active_video_settings_live();
                    return true;
                }
            }
            if let Some(ActiveScreen::SkinCustomization(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::ChatOptions(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::SoundOptions(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::WorldOptions(screen)) = self.active_screen.as_mut() {
                screen.mouse_scrolled(scaled_y, logical_h.round() as i32);
                return true;
            }
            if let Some(ActiveScreen::Controls(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::CreditsAndAttribution(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::Multiplayer(screen)) = self.active_screen.as_mut() {
                let handled = Screen::mouse_scrolled(
                    screen,
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::Restrictions(screen)) = self.active_screen.as_mut() {
                screen.mouse_scrolled(scaled_y, logical_h.round() as i32, &self.font);
                return true;
            }
            if let Some(ActiveScreen::AccessibilityOptions(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::MouseSettings(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::LanguageSelect(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::ResourcePacks(screen)) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
            if let Some(ActiveScreen::KeyBinds { screen, .. }) = self.active_screen.as_mut() {
                use el_gui::framework::screen::Screen;
                let handled = screen.mouse_scrolled(
                    self.mouse_x as f64,
                    self.mouse_y as f64,
                    0.0,
                    scaled_y as f64,
                );
                if handled {
                    return true;
                }
            }
        }
        let screen = self.active_screen.take();
        let Some(mut scr) = screen else {
            return false;
        };
        let mut keep = true;
        let mut pending_server_dialog_link_prompt: Option<(String, bool)> = None;
        let handled = match &mut scr {
            ActiveScreen::Title(title_screen) => {
                title_screen.set_extensions(
                    self.extension_catalog.mods_enabled,
                    self.extension_catalog.shaderpacks_enabled,
                );
                let handled = route_migrated_screen(
                    title_screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = title_screen.take_action() {
                    if let TitleScreenAction::OpenExtensionCatalog(kind) = action {
                        self.open_extension_catalog(
                            kind,
                            ActiveScreen::Title(title_screen.clone()),
                        );
                    } else {
                        self.handle_title_action(action);
                    }
                    keep = false;
                }
                handled
            }
            ActiveScreen::NavigationTransition => {
                keep = false;
                true
            }
            ActiveScreen::Container(screen) => {
                let has_shift = self.shift_held || controls.is_down(crate::input::KeyAction::Sneak);
                let keyboard_action = match event {
                    WindowEvent::Ime(ime) if screen.handle_ime_event(ime, &self.font) => {
                        crate::ui::gui::screen::container::ContainerKeyboardAction::Handled
                    }
                    WindowEvent::Ime(_) => {
                        crate::ui::gui::screen::container::ContainerKeyboardAction::Unhandled
                    }
                    _ => screen.handle_keyboard_event(
                        event,
                        controls,
                        self.control_held,
                        self.shift_held,
                        &self.font,
                        &mut *self.clipboard.borrow_mut(),
                    ),
                };
                let keyboard_handled = match keyboard_action {
                    crate::ui::gui::screen::container::ContainerKeyboardAction::Unhandled => false,
                    crate::ui::gui::screen::container::ContainerKeyboardAction::Handled => true,
                    crate::ui::gui::screen::container::ContainerKeyboardAction::Close => {
                        screen.on_close();
                        keep = false;
                        self.requested_container_close = Some(screen.container_id);
                        true
                    }
                    crate::ui::gui::screen::container::ContainerKeyboardAction::Clicks(pkts) => {
                        self.pending_container_clicks.extend(pkts);
                        true
                    }
                };
                if !keyboard_handled {
                    if pressed_key(event, KeyCode::Escape) {
                        screen.on_close();
                        keep = false;
                        self.requested_container_close = Some(screen.container_id);
                    } else if let WindowEvent::MouseInput { state, button, .. } = event {
                        let is_pressed = *state == ElementState::Pressed;
                        if is_pressed {
                            if let Some(pkts) = screen.handle_mouse_click(
                                self.mouse_x,
                                self.mouse_y,
                                *button,
                                false,
                                has_shift,
                            ) {
                                self.pending_container_clicks.extend(pkts);
                            }
                        } else if let Some(pkts) = screen.handle_mouse_release(
                            self.mouse_x,
                            self.mouse_y,
                            *button,
                            has_shift,
                        ) {
                            self.pending_container_clicks.extend(pkts);
                        }
                    }
                }
                for slot in &screen.menu.slots {
                    if slot.container_idx == 0 {
                        let item = screen.menu.containers[0].get_item(slot.slot_idx);
                        if slot.slot_idx < player.inventory.len() {
                            player.inventory[slot.slot_idx] = item.clone();
                        }
                    }
                }
                if !screen.pending_creative_clicks.is_empty() {
                    self.pending_creative_clicks
                        .extend(screen.pending_creative_clicks.drain(..));
                }
                self.pending_bundle_item_selections
                    .extend(screen.take_pending_bundle_item_selections());
                self.pending_container_slot_state_changes
                    .extend(screen.take_pending_slot_state_changes());
                true
            }
            ActiveScreen::Pause(pause_screen) => {
                pause_screen.set_extensions(
                    self.extension_catalog.mods_enabled,
                    self.extension_catalog.shaderpacks_enabled,
                );
                let (available, title) = self.pause_screen_addition();
                pause_screen.set_custom_addition(available, title);
                let handled = route_migrated_screen(
                    pause_screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = pause_screen.take_action() {
                    if let PauseMenuAction::OpenExtensionCatalog(kind) = action {
                        self.open_extension_catalog(
                            kind,
                            ActiveScreen::Pause(pause_screen.clone()),
                        );
                    } else if let PauseMenuAction::OpenSubmenu(submenu) = action {
                        if matches!(
                            submenu,
                            PauseSubmenu::Stats
                                | PauseSubmenu::Advancements
                                | PauseSubmenu::ServerLinks
                        ) {
                            let parent = std::mem::replace(
                                pause_screen,
                                PauseMenuScreen::new(
                                    true,
                                    self.has_singleplayer_server,
                                    self.is_local_server,
                                    std::sync::Arc::clone(&self.font)
                                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                ),
                            );
                            self.menu_parent_stack.push(ActiveScreen::Pause(parent));
                            match submenu {
                                PauseSubmenu::Stats => self.open_stats_screen(),
                                PauseSubmenu::Advancements => self.open_advancements_screen(),
                                PauseSubmenu::ServerLinks => {
                                    if let Some((kind, Some(data), _)) =
                                        self.pause_screen_dialog.clone()
                                    {
                                        self.active_screen = self.menu_parent_stack.pop();
                                        self.show_server_dialog(Some(kind), Some(data), true);
                                    } else {
                                        self.active_screen = Some(ActiveScreen::ServerLinks(
                                            ServerLinksScreen::new(
                                                true,
                                                self.server_links.clone(),
                                                Arc::clone(&self.font)
                                                    as Arc<dyn el_gui::FontWidthMeasurer>,
                                            ),
                                        ));
                                    }
                                }
                                _ => unreachable!("only data submenus are handled here"),
                            }
                        } else if let Some(url) = pause_external_link_url(submenu) {
                            let parent = std::mem::replace(
                                pause_screen,
                                PauseMenuScreen::new(
                                    true,
                                    self.has_singleplayer_server,
                                    self.is_local_server,
                                    std::sync::Arc::clone(&self.font)
                                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                ),
                            );
                            self.menu_parent_stack.push(ActiveScreen::Pause(parent));
                            self.active_screen =
                                Some(ActiveScreen::ConfirmLink(ConfirmLinkScreen::new(
                                    true,
                                    url.to_string(),
                                    true,
                                    std::sync::Arc::clone(&self.font)
                                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                )));
                        } else {
                            self.handle_pause_action(PauseMenuAction::OpenSubmenu(submenu));
                        }
                    } else if action == PauseMenuAction::ConfirmFriendsList {
                        let parent = std::mem::replace(
                            pause_screen,
                            PauseMenuScreen::new(
                                true,
                                self.has_singleplayer_server,
                                self.is_local_server,
                                std::sync::Arc::clone(&self.font)
                                    as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                            ),
                        );
                        let prompt = FriendsListConfirmScreen::new(
                            true,
                            std::sync::Arc::clone(&self.font)
                                as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                        );
                        self.active_screen = Some(ActiveScreen::FriendsConfirm {
                            screen: prompt,
                            parent: Some(Box::new(ActiveScreen::Pause(parent))),
                            in_world: true,
                        });
                    } else {
                        self.handle_pause_action(action);
                    }
                    keep = false;
                }
                handled
            }
            ActiveScreen::ExtensionCatalog { screen, parent } => {
                let scale =
                    self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
                let (logical_w, logical_h) =
                    (self.last_screen_w / scale, self.last_screen_h / scale);
                let action = self.extension_catalog_event(screen, event, logical_w, logical_h);
                if let Some(action) = action {
                    match action {
                        ExtensionCatalogAction::Back => {
                            self.active_screen = parent.take().map(|parent| *parent);
                            keep = false;
                        }
                        ExtensionCatalogAction::OpenFolder(kind) => {
                            let directory = self.game_directory().join(match kind {
                                ExtensionCatalogKind::Mods => "mods",
                                ExtensionCatalogKind::ShaderPacks => "shaderpacks",
                            });
                            match std::fs::create_dir_all(&directory) {
                                Ok(()) => open_file_cross_platform(&directory.to_string_lossy()),
                                Err(error) => {
                                    log::warn!("Could not open extension folder: {error}")
                                }
                            }
                        }
                        ExtensionCatalogAction::OpenSettings(owner) => {
                            self.pending_extension_settings_request = Some(owner.clone());
                            extension_menus::wrap_in_extension_settings(&mut scr, owner);
                            self.mark_screen_dirty();
                            keep = true;
                        }
                    }
                }
                true
            }
            ActiveScreen::ExtensionSettings { parent, .. } => {
                let scale =
                    self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
                let (logical_w, logical_h) =
                    (self.last_screen_w / scale, self.last_screen_h / scale);
                if self.extension_settings_back(event, logical_w, logical_h) {
                    self.active_screen = parent.take().map(|parent| *parent);
                    keep = false;
                }
                true
            }
            ActiveScreen::Options(options_screen) => {
                let handled = route_migrated_screen(
                    options_screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(raw) = options_screen.take_fov_drag() {
                    let degrees = el_gui::settings::options::fov_degrees_from_slider(raw);
                    self.fov_setting = el_gui::settings::options::fov_slider_from_degrees(degrees);
                }
                match options_screen.take_action() {
                    Some(OptionsAction::OpenSubmenu(submenu)) => {
                        let in_world = options_screen.in_world();
                        self.save_options();
                        self.stash_current_menu_parent(&mut scr);
                        self.open_options_submenu(submenu, in_world);
                        keep = false;
                    }
                    Some(OptionsAction::Done) => {
                        self.save_options();
                        if let Some(parent) = self.menu_parent_stack.pop() {
                            self.active_screen = Some(parent);
                        } else {
                            let in_world = options_screen.in_world();
                            if in_world {
                                self.active_screen =
                                    Some(ActiveScreen::Pause(PauseMenuScreen::new(
                                        true,
                                        self.has_singleplayer_server,
                                        self.is_local_server,
                                        std::sync::Arc::clone(&self.font)
                                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                    )));
                            } else {
                                self.open_title_screen();
                            }
                        }
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::Video(video_screen) => {
                video_screen.set_shaderpacks_enabled(self.extension_catalog.shaderpacks_enabled);
                let handled = route_migrated_screen(
                    video_screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if video_screen.take_value_changed() {
                    self.apply_video_settings(video_screen);
                    self.save_options();
                }
                if let Some(action) = video_screen.take_action() {
                    match action {
                        VideoSettingsAction::Done => {
                            self.apply_video_settings(video_screen);
                            self.commit_fullscreen_video_mode_change();
                            let in_world = video_screen.in_world();
                            self.active_screen = Some(self.menu_parent_or_options(in_world));
                            self.save_options();
                        }
                        VideoSettingsAction::GuiScaleChanged => {
                            self.apply_video_settings(video_screen);
                            let old_scale = self.gui_scale_setting;
                            self.gui_scale_setting = video_screen
                                .options
                                .iter()
                                .find(|o| o.key == "guiScale")
                                .map(|o| o.value.round() as u32)
                                .unwrap_or(self.gui_scale_setting);
                            if self.gui_scale_setting != old_scale {
                                self.update_logical_mouse();
                                self.save_options();
                            }
                            let mut replacement = ActiveScreen::Video(
                                self.video_settings_screen(video_screen.in_world()),
                            );
                            std::mem::swap(&mut scr, &mut replacement);
                            self.active_screen = Some(replacement);
                        }
                        VideoSettingsAction::OpenShaderPacks => {
                            self.apply_video_settings(video_screen);
                            let mut parent = ActiveScreen::Video(
                                self.video_settings_screen(video_screen.in_world()),
                            );
                            std::mem::swap(&mut scr, &mut parent);
                            self.open_extension_catalog(ExtensionCatalogKind::ShaderPacks, parent);
                        }
                    }
                    keep = false;
                }
                handled
            }
            ActiveScreen::SkinCustomization(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if screen.take_value_changed() {
                    self.apply_skin_customization_settings(screen);
                    self.save_options();
                }
                if let Some(action) = screen.take_action() {
                    match action {
                        SkinCustomizationAction::Done => {
                            self.apply_skin_customization_settings(screen);
                            self.active_screen = Some(self.menu_parent_or_options(screen.in_world));
                            self.save_options();
                        }
                    }
                    keep = false;
                }
                handled
            }
            ActiveScreen::ChatOptions(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if handled {
                    self.apply_chat_options(screen);
                    self.chat_component.rescale_chat();
                }
                if screen.take_action().is_some() {
                    self.apply_chat_options(screen);
                    let in_world = screen.in_world();
                    self.active_screen = Some(self.menu_parent_or_options(in_world));
                    self.save_options();
                    keep = false;
                }
                handled
            }
            ActiveScreen::SoundOptions(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                let selected_output = screen
                    .take_audio_output_changed()
                    .then(|| screen.selected_audio_output_handle().map(str::to_owned))
                    .flatten();
                if screen.take_value_changed() {
                    let options = screen.options.clone();
                    self.apply_sound_option_values(&options);
                }
                if let Some(handle) = selected_output {
                    self.queue_audio_output_selection(handle);
                }
                if let Some(action) = screen.take_action() {
                    match action {
                        SoundOptionsAction::Done => {
                            self.apply_sound_options_settings(screen);
                            self.active_screen = Some(self.menu_parent_or_options(screen.in_world));
                            self.save_options();
                        }
                    }
                    keep = false;
                }
                handled
            }
            ActiveScreen::OnlineOptions(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = screen.take_action() {
                    match action {
                        OnlineOptionsAction::Done => {
                            self.save_options();
                            self.active_screen =
                                Some(self.menu_parent_or_options(screen.in_world()));
                            keep = false;
                        }
                        OnlineOptionsAction::AllowServerListingChanged(enabled) => {
                            self.set_client_information_allows_listing(enabled);
                        }
                        OnlineOptionsAction::PrivacySettingsLinkClicked => {
                            let in_world = screen.in_world();
                            self.stash_current_menu_parent(&mut scr);
                            self.active_screen =
                                Some(ActiveScreen::ConfirmLink(ConfirmLinkScreen::new(
                                    in_world,
                                    "https://aka.ms/MinecraftJavaXboxPrivacyAndSafety".into(),
                                    true,
                                    std::sync::Arc::clone(&self.font)
                                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                )));
                            keep = false;
                        }
                    }
                }
                handled
            }
            ActiveScreen::TelemetryInfo(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = screen.take_action() {
                    match action {
                        TelemetryInfoAction::Done => {
                            self.active_screen =
                                Some(self.menu_parent_or_options(screen.in_world()));
                            keep = false;
                        }
                        TelemetryInfoAction::PrivacyStatementRequested => {
                            let in_world = screen.in_world();
                            self.stash_current_menu_parent(&mut scr);
                            self.active_screen =
                                Some(ActiveScreen::ConfirmLink(ConfirmLinkScreen::new(
                                    in_world,
                                    "http://go.microsoft.com/fwlink/?LinkId=521839".into(),
                                    true,
                                    std::sync::Arc::clone(&self.font)
                                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                )));
                            keep = false;
                        }
                        TelemetryInfoAction::FeedbackRequested => {
                            let in_world = screen.in_world();
                            self.stash_current_menu_parent(&mut scr);
                            self.active_screen =
                                Some(ActiveScreen::ConfirmLink(ConfirmLinkScreen::new(
                                    in_world,
                                    "https://aka.ms/javafeedback?ref=game".into(),
                                    true,
                                    std::sync::Arc::clone(&self.font)
                                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                )));
                            keep = false;
                        }
                        TelemetryInfoAction::OptionalTelemetryConsentChanged(_)
                        | TelemetryInfoAction::TelemetryLogDirectoryRequested => {
                            log::warn!(
                                "Telemetry screen emitted an action for an unavailable service"
                            );
                        }
                    }
                }
                handled
            }
            ActiveScreen::ServerLinks(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = screen.take_action() {
                    match action {
                        ServerLinksScreenAction::Done => {
                            self.active_screen = self.menu_parent_stack.pop();
                            keep = false;
                        }
                        ServerLinksScreenAction::OpenLink(url) => {
                            if is_allowed_untrusted_chat_url(&url)
                                && self.option_bool("chatLinks", true)
                            {
                                if self.option_bool("chatLinksPrompt", true) {
                                    let in_world = screen.in_world();
                                    self.stash_current_menu_parent(&mut scr);
                                    self.active_screen =
                                        Some(ActiveScreen::ConfirmLink(ConfirmLinkScreen::new(
                                            in_world,
                                            url,
                                            false,
                                            std::sync::Arc::clone(&self.font)
                                                as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                        )));
                                    keep = false;
                                } else {
                                    open_url_cross_platform(url.trim());
                                }
                            } else if !is_allowed_untrusted_chat_url(&url) {
                                log::warn!("Blocked server link with invalid http(s) URL scheme");
                            }
                        }
                    }
                }
                handled
            }
            ActiveScreen::ServerDialog {
                screen,
                parent,
                in_world: _,
            } => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(request) = screen.take_request() {
                    let after_action = request.after_action;
                    if let Some(action) = request.action {
                        match action {
                            DialogClickAction::RunCommand { command, dynamic } => {
                                let command = if dynamic {
                                    instantiate_dialog_template(&command, &request.inputs)
                                } else {
                                    Some(command)
                                };
                                if let Some(command) = command {
                                    self.pending_unattended_chat_commands.push(
                                        command.strip_prefix('/').unwrap_or(&command).to_string(),
                                    );
                                }
                            }
                            DialogClickAction::OpenUrl(url) => {
                                if is_allowed_untrusted_chat_url(&url)
                                    && self.option_bool("chatLinks", true)
                                {
                                    if self.option_bool("chatLinksPrompt", true) {
                                        if after_action == DialogAfterAction::WaitForResponse {
                                            screen.begin_waiting_for_response();
                                        }
                                        pending_server_dialog_link_prompt =
                                            Some((url, after_action != DialogAfterAction::Close));
                                        keep = false;
                                    } else {
                                        open_url_cross_platform(url.trim());
                                    }
                                } else if !is_allowed_untrusted_chat_url(&url) {
                                    log::warn!(
                                        "Blocked server dialog URL with invalid http(s) URL scheme"
                                    );
                                }
                            }
                            DialogClickAction::CopyToClipboard(value) => {
                                el_gui::components::edit_box::Clipboard::set(
                                    &mut *self.clipboard.borrow_mut(),
                                    value.clone(),
                                );
                                self.chat_clipboard = value;
                            }
                            DialogClickAction::SuggestCommand(command) => {
                                if request.after_action != DialogAfterAction::KeepOpen
                                    && let Some(parent) = parent.as_mut()
                                    && let ActiveScreen::Chat { input } = parent.as_mut()
                                {
                                    *input = command;
                                }
                            }
                            DialogClickAction::Custom {
                                id,
                                payload: base_payload,
                                dynamic_inputs,
                            } => {
                                match build_dialog_custom_payload(
                                    base_payload.as_deref(),
                                    dynamic_inputs,
                                    &request.inputs,
                                ) {
                                    Ok(payload) => {
                                        self.pending_custom_click_actions.push((id, payload));
                                    }
                                    Err(reason) => {
                                        log::warn!(
                                            "Could not build server dialog custom-action payload: {reason}"
                                        );
                                    }
                                }
                            }
                            DialogClickAction::OpenDialog(_) => {}
                            DialogClickAction::LocalSignUpdate { pos, is_front_text } => {
                                let lines = std::array::from_fn(|index| {
                                    let key = format!("line{index}");
                                    let value = match request.inputs.get(&key) {
                                        Some(DialogInputValue::Text(value)) => value.as_str(),
                                        _ => "",
                                    };
                                    let mut fitted = String::new();
                                    for character in value.chars() {
                                        fitted.push(character);
                                        if self.font.measure_width(&fitted, 1.0) > 90.0 {
                                            fitted.pop();
                                            break;
                                        }
                                    }
                                    fitted
                                });
                                self.pending_sign_update = Some((pos, is_front_text, lines));
                            }
                        }
                    }
                    if pending_server_dialog_link_prompt.is_none() {
                        match after_action {
                            DialogAfterAction::KeepOpen => {}
                            DialogAfterAction::Close => {
                                self.active_screen = parent.take().map(|parent| *parent);
                                keep = false;
                            }
                            DialogAfterAction::WaitForResponse => {
                                screen.begin_waiting_for_response();
                            }
                        }
                    }
                }
                handled
            }
            ActiveScreen::WorldOptions(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                match screen.take_action() {
                    Some(WorldOptionsAction::Done) => {
                        if screen.can_edit_world_data() {
                            self.apply_world_options_settings(screen);
                        }
                        let in_world = screen.in_world();
                        self.active_screen = Some(self.menu_parent_or_options(in_world));
                        self.save_options();
                        keep = false;
                    }
                    Some(WorldOptionsAction::GameRulesClicked) => {
                        let in_world = screen.in_world();
                        self.stash_current_menu_parent(&mut scr);
                        if self.supports_game_rule_editor() {
                            self.open_game_rules_screen(in_world);
                        } else {
                            self.active_screen = Some(ActiveScreen::SubmenuStub {
                                submenu: PauseSubmenu::GameRulesUnavailable,
                                in_world,
                            });
                        }
                        keep = false;
                    }
                    Some(WorldOptionsAction::RestrictionsClicked) => {
                        let in_world = screen.in_world();
                        let child =
                            ActiveScreen::Restrictions(self.chat_restrictions_screen(in_world));
                        self.stash_current_menu_parent(&mut scr);
                        self.active_screen = Some(child);
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::GameRules(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = screen.take_action() {
                    let in_world = screen.in_world();
                    if action == GameRulesAction::Done {
                        self.pending_game_rule_edits.extend(
                            screen
                                .changed_rows()
                                .into_iter()
                                .filter(|row| row.is_editable())
                                .filter_map(|row| {
                                    let value = match row.value {
                                        GameRuleValue::Boolean(value) => value.to_string(),
                                        GameRuleValue::Integer(value) => value.to_string(),
                                        GameRuleValue::Decimal(value) if value.is_finite() => {
                                            value.to_string()
                                        }
                                        GameRuleValue::String(value) => value,
                                        GameRuleValue::Decimal(_) => return None,
                                    };
                                    Some((row.key, value))
                                }),
                        );
                    }
                    self.active_screen = self.menu_parent_stack.pop().or_else(|| {
                        Some(ActiveScreen::WorldOptions(
                            self.world_options_screen(in_world),
                        ))
                    });
                    keep = false;
                }
                handled
            }
            ActiveScreen::Controls(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if screen.take_value_changed() {
                    self.apply_controls_settings(screen);
                    self.save_options();
                }
                if let Some(action) = screen.take_action() {
                    let closes_controls = match action {
                        ControlsAction::Done => {
                            self.apply_controls_settings(screen);
                            self.active_screen = Some(self.menu_parent_or_options(screen.in_world));
                            self.save_options();
                            true
                        }
                        ControlsAction::MouseSettingsClicked => {
                            let child = ActiveScreen::MouseSettings(MouseSettingsScreen::new(
                                screen.in_world,
                                &self.extra_options,
                                self.sensitivity_setting,
                                f64::from(self.mouse_wheel_sensitivity),
                                std::sync::Arc::clone(&self.font)
                                    as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                            ));
                            self.stash_current_menu_parent(&mut scr);
                            self.active_screen = Some(child);
                            true
                        }
                        ControlsAction::KeybindsClicked => {
                            let entries = build_keybind_entries(controls);
                            let child = ActiveScreen::KeyBinds {
                                screen: KeyBindsScreen::new(
                                    screen.in_world,
                                    entries,
                                    std::sync::Arc::clone(&self.font)
                                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                ),
                                waiting_for_key: None,
                            };
                            self.stash_current_menu_parent(&mut scr);
                            self.active_screen = Some(child);
                            true
                        }
                        ControlsAction::OptionChanged => {
                            self.apply_controls_settings(screen);
                            self.save_options();
                            false
                        }
                    };
                    if closes_controls {
                        keep = false;
                    }
                }
                handled
            }
            ActiveScreen::Multiplayer(screen) => {
                use crate::network::multiplayer::ServerList;
                let scale =
                    self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
                let logical_h = self.last_screen_h / scale;
                let visible_h = logical_h - 93.0;
                let scroll_to_row = |screen: &mut MultiplayerScreen, idx: usize| {
                    let top = (idx * SERVER_ROW_HEIGHT_PX) as f32;
                    if top < screen.scroll_y {
                        screen.scroll_y = top;
                    }
                    let min_scroll_for_bottom = top + 40.0 - visible_h;
                    if screen.scroll_y < min_scroll_for_bottom {
                        screen.scroll_y = min_scroll_for_bottom
                            .max(0.0)
                            .min(screen.max_scroll_amount(logical_h.round() as i32));
                    }
                };
                let swap_servers = |game_dir: std::path::PathBuf,
                                    screen: &mut MultiplayerScreen,
                                    a: usize,
                                    b: usize| {
                    if a >= screen.servers.len() || b >= screen.servers.len() {
                        return;
                    }
                    screen.servers.swap(a, b);
                    let mut list = ServerList::new(game_dir);
                    list.load();
                    if a < list.servers.len() && b < list.servers.len() {
                        list.servers.swap(a, b);
                        list.save();
                    }
                };
                if pressed_key(event, KeyCode::F5) {
                    self.enter_multiplayer_screen(screen.in_world);
                    keep = false;
                    true
                } else if pressed_key(event, KeyCode::ArrowUp) {
                    let selected = screen.selected_entry_ordinal();
                    let new_ordinal = if self.shift_held
                        && let Some(index) = screen.selected_index
                        && index > 0
                    {
                        swap_servers(self.game_directory(), screen, index, index - 1);
                        Some(index - 1)
                    } else {
                        match selected {
                            Some(0) => None,
                            Some(ordinal) => Some(ordinal - 1),
                            None => screen.entry_count().checked_sub(1),
                        }
                    };
                    screen.select_entry_ordinal(new_ordinal);
                    if let Some(ordinal) = new_ordinal {
                        scroll_to_row(screen, ordinal);
                    }
                    true
                } else if pressed_key(event, KeyCode::ArrowDown) {
                    let selected = screen.selected_entry_ordinal();
                    let new_ordinal = if self.shift_held
                        && let Some(index) = screen.selected_index
                        && index + 1 < screen.servers.len()
                    {
                        swap_servers(self.game_directory(), screen, index, index + 1);
                        Some(index + 1)
                    } else {
                        match selected {
                            Some(ordinal) if ordinal + 1 < screen.entry_count() => {
                                Some(ordinal + 1)
                            }
                            Some(_) => None,
                            None => Some(0),
                        }
                    };
                    screen.select_entry_ordinal(new_ordinal);
                    if let Some(ordinal) = new_ordinal {
                        scroll_to_row(screen, ordinal);
                    }
                    true
                } else if (pressed_key(event, KeyCode::Enter)
                    || pressed_key(event, KeyCode::NumpadEnter)
                    || pressed_key(event, KeyCode::Space))
                    && screen.list_focused
                {
                    if let Some(idx) = screen.selected_index {
                        if idx < screen.servers.len() {
                            let s = &screen.servers[idx];
                            let server_name = s.name.clone();
                            let server_ip = s.ip.clone();
                            self.stashed_multiplayer_screen = Some(screen.clone());
                            self.start_server_connection(screen.in_world, server_name, server_ip);
                            keep = false;
                        }
                    } else if let Some(idx) = screen.selected_lan_index {
                        if idx < screen.lan_servers.len() {
                            let lan = &screen.lan_servers[idx];
                            let server_name = lan.motd.clone();
                            let server_ip = lan.address.clone();
                            self.stashed_multiplayer_screen = Some(screen.clone());
                            self.start_server_connection(screen.in_world, server_name, server_ip);
                            keep = false;
                        }
                    }
                    true
                } else {
                    let handled = route_migrated_screen(
                        screen,
                        event,
                        self.mouse_x,
                        self.mouse_y,
                        self.shift_held,
                        self.control_held,
                    );
                    while let Some(action) = screen.take_action() {
                        match action {
                            MultiplayerAction::Back => {
                                self.open_title_screen();
                                keep = false;
                            }
                            MultiplayerAction::Join(idx) => {
                                if idx < screen.servers.len() {
                                    let s = &screen.servers[idx];
                                    let in_world = screen.in_world;
                                    let server_name = s.name.clone();
                                    let server_ip = s.ip.clone();
                                    self.stashed_multiplayer_screen = Some(screen.clone());
                                    self.start_server_connection(in_world, server_name, server_ip);
                                    self.stop_lan_server_detector();
                                    keep = false;
                                }
                            }
                            MultiplayerAction::JoinLan(idx) => {
                                if idx < screen.lan_servers.len() {
                                    let lan = &screen.lan_servers[idx];
                                    let in_world = screen.in_world;
                                    let server_name = lan.motd.clone();
                                    let server_ip = lan.address.clone();
                                    self.stashed_multiplayer_screen = Some(screen.clone());
                                    self.start_server_connection(in_world, server_name, server_ip);
                                    self.stop_lan_server_detector();
                                    keep = false;
                                }
                            }
                            MultiplayerAction::DirectConnect => {
                                self.stashed_multiplayer_screen = Some(screen.clone());
                                crate::request_android_keyboard();
                                let last_ip = self
                                    .extra_options
                                    .get("lastMpIp")
                                    .cloned()
                                    .unwrap_or_default();
                                let last_ip: String = last_ip
                                    .chars()
                                    .filter(|c| !c.is_control() && *c != '\0')
                                    .collect();
                                let mut direct_connect = DirectJoinServerScreen::new(
                                    screen.in_world,
                                    last_ip,
                                    std::sync::Arc::clone(&self.font)
                                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                );
                                direct_connect.set_clipboard(self.clipboard.clone());
                                self.active_screen =
                                    Some(ActiveScreen::DirectConnect(direct_connect));
                                keep = false;
                            }
                            MultiplayerAction::AddServer => {
                                ui_crash_trace(format!(
                                    "AddServer stage=action in_world={} servers={} size={}x{}",
                                    screen.in_world,
                                    screen.servers.len(),
                                    screen.base_ref().width,
                                    screen.base_ref().height,
                                ));
                                ui_crash_trace("AddServer stage=parent_clone_begin");
                                self.stashed_multiplayer_screen = Some(screen.clone());
                                ui_crash_trace("AddServer stage=parent_clone_done");
                                crate::request_android_keyboard();
                                ui_crash_trace("AddServer stage=screen_construction_begin");
                                let mut manage_server = ManageServerScreen::new(
                                    screen.in_world,
                                    None,
                                    "".to_string(),
                                    "".to_string(),
                                    ResourcePackStatus::Prompt,
                                    false,
                                    std::sync::Arc::clone(&self.font)
                                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                );
                                ui_crash_trace("AddServer stage=screen_construction_done");
                                manage_server.set_clipboard(self.clipboard.clone());
                                self.active_screen =
                                    Some(ActiveScreen::ManageServer(manage_server));
                                ui_crash_trace("AddServer stage=screen_installed");
                                keep = false;
                            }
                            MultiplayerAction::Edit(idx) => {
                                if idx < screen.servers.len() {
                                    let s = &screen.servers[idx];
                                    let server_name = s.name.clone();
                                    let server_ip = s.ip.clone();
                                    let pack_status = {
                                        let mut list = ServerList::new(self.game_directory());
                                        list.load();
                                        list.get_by_ip(&server_ip).map(|sd| sd.pack_status)
                                    };
                                    let pack_status = match pack_status {
                                        Some(
                                            crate::network::multiplayer::server_data::ServerPackStatus::Enabled,
                                        ) => ResourcePackStatus::Enabled,
                                        Some(
                                            crate::network::multiplayer::server_data::ServerPackStatus::Disabled,
                                        ) => ResourcePackStatus::Disabled,
                                        Some(
                                            crate::network::multiplayer::server_data::ServerPackStatus::Prompt,
                                        )
                                        | None => ResourcePackStatus::Prompt,
                                    };
                                    self.stashed_multiplayer_screen = Some(screen.clone());
                                    let mut manage_server = ManageServerScreen::new(
                                        screen.in_world,
                                        Some(idx),
                                        server_name,
                                        server_ip,
                                        pack_status,
                                        true,
                                        std::sync::Arc::clone(&self.font)
                                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                    );
                                    manage_server.set_clipboard(self.clipboard.clone());
                                    self.active_screen =
                                        Some(ActiveScreen::ManageServer(manage_server));
                                    keep = false;
                                }
                            }
                            MultiplayerAction::Delete(idx) => {
                                if idx < screen.servers.len() {
                                    let server_name = screen.servers[idx].name.clone();
                                    let raw_warning =
                                        el_gui::translate("selectServer.deleteWarning");
                                    let msg = raw_warning.replace("%s", &server_name);
                                    self.stashed_multiplayer_screen = Some(screen.clone());
                                    self.active_screen =
                                        Some(ActiveScreen::Confirm(ConfirmScreen::new(
                                            screen.in_world,
                                            "selectServer.deleteQuestion".to_string(),
                                            msg,
                                            "selectServer.deleteButton".to_string(),
                                            "gui.cancel".to_string(),
                                            ConfirmContext::DeleteServer(idx),
                                            std::sync::Arc::clone(&self.font)
                                                as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                        )));
                                    keep = false;
                                }
                            }
                            MultiplayerAction::Refresh => {
                                self.enter_multiplayer_screen(screen.in_world);
                                keep = false;
                            }
                            MultiplayerAction::Swap(idx1, idx2) => {
                                swap_servers(self.game_directory(), screen, idx1, idx2);
                                if let Some(sel) = screen.selected_index {
                                    if sel == idx1 {
                                        screen.selected_index = Some(idx2);
                                    } else if sel == idx2 {
                                        screen.selected_index = Some(idx1);
                                    }
                                }
                                if let Some(sel) = screen.selected_index {
                                    scroll_to_row(screen, sel);
                                }
                            }
                        }
                    }
                    handled
                }
            }
            ActiveScreen::ManageServer(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                match screen.take_action() {
                    Some(ManageServerAction::Cancel) => {
                        let in_world = screen.in_world();
                        self.restore_multiplayer_screen(in_world);
                        keep = false;
                    }
                    Some(ManageServerAction::Done {
                        name,
                        ip,
                        pack_status,
                    }) => {
                        let in_world = screen.in_world();
                        let edit_index = screen.edit_index();
                        self.apply_manage_server_done(in_world, edit_index, name, ip, pack_status);
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::DirectConnect(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = screen.take_action() {
                    match action {
                        DirectJoinServerAction::Cancel => {
                            let clean_ip: String = screen
                                .server_ip
                                .chars()
                                .filter(|c| !c.is_control() && *c != '\0')
                                .collect();
                            self.extra_options.insert("lastMpIp".to_string(), clean_ip);
                            self.save_options();
                            let in_world = screen.in_world;
                            self.restore_multiplayer_screen(in_world);
                            keep = false;
                        }
                        DirectJoinServerAction::Done { ip } => {
                            self.extra_options
                                .insert("lastMpIp".to_string(), ip.clone());
                            self.save_options();
                            let in_world = screen.in_world;
                            let server_name = self.direct_connect_server_name(&ip);
                            self.start_server_connection(in_world, server_name, ip);
                            keep = false;
                        }
                    }
                }
                handled
            }
            ActiveScreen::Disconnected(screen) => {
                if route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                ) {
                    match screen.take_action() {
                        Some(DisconnectedAction::Back) => {
                            if screen.allows_multiplayer {
                                let in_world = screen.in_world;
                                self.restore_multiplayer_screen(in_world);
                            } else {
                                self.open_title_screen();
                            }
                            keep = false;
                        }
                        Some(DisconnectedAction::OpenLink(link)) => {
                            let _ = std::process::Command::new("xdg-open").arg(&link).spawn();
                        }
                        Some(DisconnectedAction::OpenPath(path)) => {
                            if let Some(parent) = std::path::Path::new(&path).parent() {
                                let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                            }
                        }
                        None => {}
                    }
                    true
                } else {
                    false
                }
            }
            ActiveScreen::Safety(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                match screen.take_action() {
                    Some(SafetyScreenAction::Back) => {
                        self.open_title_screen();
                        keep = false;
                    }
                    Some(SafetyScreenAction::Proceed { skip_warning }) => {
                        if skip_warning {
                            self.extra_options
                                .insert("skipMultiplayerWarning".to_string(), "true".to_string());
                            self.save_options();
                        }
                        let in_world = screen.in_world;
                        self.enter_multiplayer_screen(in_world);
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::CodeOfConduct(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                match screen.take_action() {
                    Some(CodeOfConductAction::Disconnect) => {
                        let text = screen.code_of_conduct_text.clone();
                        self.apply_code_of_conduct_result(false, false, &text);
                        self.requested_disconnect = true;
                        self.active_screen = None;
                        keep = false;
                    }
                    Some(CodeOfConductAction::Acknowledge { skip_next_time }) => {
                        let text = screen.code_of_conduct_text.clone();
                        self.apply_code_of_conduct_result(true, skip_next_time, &text);
                        self.close_code_of_conduct_acknowledged();
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::ConfirmLink(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                match screen.take_action() {
                    Some(ConfirmLinkAction::Open) => {
                        let url = screen.url.clone();
                        open_url_cross_platform(&url);
                        self.return_from_link_prompt();
                        keep = false;
                    }
                    Some(ConfirmLinkAction::CopyToClipboard) => {
                        let url = screen.url.clone();
                        el_gui::components::edit_box::Clipboard::set(
                            &mut *self.clipboard.borrow_mut(),
                            url.clone(),
                        );
                        self.chat_clipboard = url;
                        self.return_from_link_prompt();
                        keep = false;
                    }
                    Some(ConfirmLinkAction::Cancel) => {
                        self.return_from_link_prompt();
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::PackConfirm(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                let required = screen.required;
                match screen.take_action() {
                    Some(PackConfirmAction::Proceed) => {
                        self.persist_server_pack_decision(true, required);
                        for request in &screen.requests {
                            self.pending_server_resource_pack_responses.push(
                                ServerResourcePackResponse::Accept {
                                    id: request.id,
                                    url: request.url.clone(),
                                    hash: request.hash.clone(),
                                    required,
                                },
                            );
                        }
                        self.active_screen = self.pending_pack_parent.take().map(|parent| *parent);
                        keep = false;
                    }
                    Some(PackConfirmAction::Decline) => {
                        self.persist_server_pack_decision(false, required);
                        for request in &screen.requests {
                            self.pending_server_resource_pack_responses.push(
                                ServerResourcePackResponse::Decline {
                                    id: request.id,
                                    required,
                                },
                            );
                        }
                        self.active_screen = self.pending_pack_parent.take().map(|parent| *parent);
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::FriendsConfirm {
                screen,
                parent,
                in_world: _,
            } => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                let close_requested = screen.base().take_close_request();
                if close_requested && screen.should_close_on_esc() {
                    self.active_screen = parent.take().map(|parent| *parent);
                    self.mark_screen_dirty();
                    keep = false;
                }
                match screen.take_action() {
                    Some(FriendsListConfirmAction::Accept(_accepted)) => {
                        self.active_screen = parent.take().map(|parent| *parent);
                        self.mark_screen_dirty();
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::Confirm(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                let close_requested = screen.base().take_close_request();
                if close_requested {
                    if screen.should_close_on_esc() {
                        if matches!(&screen.context, ConfirmContext::ResourcePackIncompatible(_)) {
                            self.active_screen = self
                                .pending_resource_packs
                                .take()
                                .map(ActiveScreen::ResourcePacks);
                        } else if matches!(&screen.context, ConfirmContext::OpenLink(_)) {
                            match self.pending_link_return_input.take() {
                                Some(input) => self.open_chat_screen_with_initial(&input),
                                None => self.active_screen = None,
                            }
                        } else if let ConfirmContext::ServerResourcePack { id, required, .. } =
                            screen.context
                        {
                            self.pending_server_resource_pack_responses
                                .push(ServerResourcePackResponse::Decline { id, required });
                            self.active_screen = None;
                        } else if matches!(&screen.context, ConfirmContext::DeathScreen) {
                            self.pending_death_screen_respawn_request = true;
                            self.active_screen = None;
                        } else {
                            let in_world = screen.in_world;
                            self.restore_multiplayer_screen(in_world);
                        }
                        keep = false;
                    }
                }
                match screen.take_action() {
                    Some(ConfirmAction::Confirm(accepted)) => match screen.context.clone() {
                        ConfirmContext::DeleteServer(idx) => {
                            let in_world = screen.in_world;
                            if accepted {
                                use crate::network::multiplayer::ServerList;
                                let mut list = ServerList::new(self.game_directory());
                                list.load();
                                let removed_ip =
                                    list.servers.get(idx).map(|server| server.ip.clone());
                                if let Some(ip) = removed_ip {
                                    list.remove(&ip);
                                    list.save();
                                    if let Some(mp) = self.stashed_multiplayer_screen.as_mut() {
                                        mp.servers.retain(|entry| entry.ip != ip);
                                        mp.remove_server_icon(&ip);
                                        mp.selected_index = None;
                                        mp.selected_lan_index = None;
                                    }
                                }
                            }
                            self.restore_multiplayer_screen(in_world);
                        }
                        ConfirmContext::ResourcePackIncompatible(pack_id) => {
                            if let Some(mut pack_screen) = self.pending_resource_packs.take() {
                                if accepted {
                                    pack_screen.select_pack_by_id(&pack_id);
                                }
                                self.active_screen = Some(ActiveScreen::ResourcePacks(pack_screen));
                            }
                            keep = false;
                        }
                        ConfirmContext::OpenLink(url) => {
                            if accepted {
                                open_url_cross_platform(&url);
                            }
                            match self.pending_link_return_input.take() {
                                Some(input) => self.open_chat_screen_with_initial(&input),
                                None => self.active_screen = None,
                            }
                        }
                        ConfirmContext::ServerResourcePack {
                            id,
                            url,
                            hash,
                            required,
                        } => {
                            if accepted {
                                self.pending_server_resource_pack_responses.push(
                                    ServerResourcePackResponse::Accept {
                                        id,
                                        url,
                                        hash,
                                        required,
                                    },
                                );
                            } else {
                                self.pending_server_resource_pack_responses
                                    .push(ServerResourcePackResponse::Decline { id, required });
                            }
                            self.active_screen = None;
                        }
                        ConfirmContext::DeathScreen => {
                            if accepted {
                                self.requested_disconnect = true;
                                self.active_screen = None;
                            } else {
                                self.pending_death_screen_respawn_request = true;
                                self.active_screen = None;
                            }
                            keep = false;
                        }
                    },
                    Some(ConfirmAction::CopyToClipboard) => match screen.context.clone() {
                        ConfirmContext::OpenLink(url) => {
                            el_gui::components::edit_box::Clipboard::set(
                                &mut *self.clipboard.borrow_mut(),
                                url.clone(),
                            );
                            self.chat_clipboard = url;
                            match self.pending_link_return_input.take() {
                                Some(input) => self.open_chat_screen_with_initial(&input),
                                None => self.active_screen = None,
                            }
                        }
                        _ => {}
                    },
                    None => {}
                }
                handled
            }
            ActiveScreen::ServerReconfig(screen) => {
                if route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                ) {
                    if let Some(ServerReconfigAction::Disconnect) = screen.take_action() {
                        self.open_multiplayer_screen(screen.in_world());
                        keep = false;
                    }
                    true
                } else {
                    false
                }
            }
            ActiveScreen::ResourcePacks(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = screen.take_action() {
                    match action {
                        ResourcePacksAction::Done => {
                            let in_world = screen.in_world;
                            let selected_entries = screen.selected.clone();
                            self.commit_resource_pack_selection(&selected_entries);
                            self.active_screen = Some(self.menu_parent_or_options(in_world));
                            keep = false;
                        }
                        ResourcePacksAction::OpenFolder => {
                            let pack_dir = self.resource_pack_directory();
                            let _ = std::fs::create_dir_all(&pack_dir);
                            #[cfg(target_os = "linux")]
                            let _ = std::process::Command::new("xdg-open")
                                .arg(&pack_dir)
                                .spawn();
                            #[cfg(target_os = "windows")]
                            let _ = std::process::Command::new("explorer")
                                .arg(&pack_dir)
                                .spawn();
                            #[cfg(target_os = "macos")]
                            let _ = std::process::Command::new("open").arg(&pack_dir).spawn();
                        }
                        ResourcePacksAction::ConfirmIncompatible(pack_id) => {
                            let in_world = screen.in_world;
                            let reason_key = screen
                                .available
                                .iter()
                                .find(|pack| pack.id == pack_id)
                                .map(|pack| pack.compatibility.confirmation_key())
                                .unwrap_or("pack.incompatible.confirm.unknown");
                            self.pending_resource_packs = Some(screen.clone());
                            self.active_screen = Some(ActiveScreen::Confirm(ConfirmScreen::new(
                                in_world,
                                "pack.incompatible.confirm.title".to_string(),
                                reason_key.to_string(),
                                "gui.yes".to_string(),
                                "gui.no".to_string(),
                                ConfirmContext::ResourcePackIncompatible(pack_id),
                                std::sync::Arc::clone(&self.font)
                                    as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                            )));
                            keep = false;
                        }
                    }
                }
                handled
            }
            ActiveScreen::Connecting(screen) => {
                if route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                ) {
                    if let Some(ConnectingAction::Cancel) = screen.take_action() {
                        let in_world = screen.in_world();
                        self.server_connection = None;
                        self.server_connection_origin = ServerConnectionOrigin::Normal;
                        self.connecting_server_ip = None;
                        self.restore_multiplayer_screen(in_world);
                        keep = false;
                    }
                    true
                } else {
                    false
                }
            }
            ActiveScreen::LevelLoading(_) => true,
            ActiveScreen::CreditsAndAttribution(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                match screen.take_action() {
                    Some(CreditsAndAttributionAction::Done) => {
                        self.apply_credits_and_attribution_settings(screen);
                        if screen.opened_from_title {
                            self.open_title_screen();
                        } else {
                            let in_world = screen.in_world;
                            self.active_screen = Some(self.menu_parent_or_options(in_world));
                            self.save_options();
                        }
                        keep = false;
                    }
                    Some(CreditsAndAttributionAction::CreditsClicked) => {
                        match self.create_win_screen(false) {
                            Ok(win_screen) => {
                                let parent =
                                    std::mem::replace(&mut scr, ActiveScreen::NavigationTransition);
                                self.active_screen = Some(ActiveScreen::WinScreen {
                                    screen: win_screen,
                                    parent: Some(Box::new(parent)),
                                });
                                keep = false;
                            }
                            Err(error) => {
                                log::error!("Couldn't open credits screen: {error}");
                            }
                        }
                    }
                    Some(CreditsAndAttributionAction::AttributionClicked) => {
                        let in_world = screen.in_world;
                        self.stash_current_menu_parent(&mut scr);
                        self.active_screen =
                            Some(ActiveScreen::ConfirmLink(ConfirmLinkScreen::new(
                                in_world,
                                "https://aka.ms/MinecraftJavaAttribution".to_string(),
                                true,
                                std::sync::Arc::clone(&self.font)
                                    as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                            )));
                        keep = false;
                    }
                    Some(CreditsAndAttributionAction::LicensesClicked) => {
                        let in_world = screen.in_world;
                        self.stash_current_menu_parent(&mut scr);
                        self.active_screen =
                            Some(ActiveScreen::ConfirmLink(ConfirmLinkScreen::new(
                                in_world,
                                "https://aka.ms/MinecraftJavaLicenses".to_string(),
                                true,
                                std::sync::Arc::clone(&self.font)
                                    as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                            )));
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::WinScreen { screen, parent } => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if screen.take_action().is_some() {
                    self.finish_win_screen(screen.poem(), parent.take());
                    keep = false;
                }
                handled
            }
            ActiveScreen::DeathScreen(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                match screen.take_action() {
                    Some(DeathScreenAction::Respawn) => {
                        self.pending_death_screen_respawn_request = true;
                    }
                    Some(DeathScreenAction::RequestTitleExit) => {
                        match screen.after_report_handled() {
                            DeathScreenAction::ExitToTitle => {
                                self.requested_disconnect = true;
                                self.active_screen = None;
                                keep = false;
                            }
                            DeathScreenAction::ShowTitleConfirmation {
                                title,
                                message,
                                yes_button,
                                no_button,
                                delay_ticks,
                            } => {
                                self.active_screen = Some(ActiveScreen::Confirm(
                                    ConfirmScreen::new(
                                        true,
                                        title,
                                        message,
                                        yes_button,
                                        no_button,
                                        ConfirmContext::DeathScreen,
                                        std::sync::Arc::clone(&self.font)
                                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                                    )
                                    .with_delay(delay_ticks as i32)
                                    .with_death_screen_background(),
                                ));
                                keep = false;
                            }
                            DeathScreenAction::Respawn
                            | DeathScreenAction::RequestTitleExit
                            | DeathScreenAction::RespawnAndClose => {
                                unreachable!("after_report_handled only exits or confirms")
                            }
                        }
                    }
                    Some(DeathScreenAction::ExitToTitle)
                    | Some(DeathScreenAction::ShowTitleConfirmation { .. })
                    | Some(DeathScreenAction::RespawnAndClose) => {
                        unreachable!("these are follow-up actions, not DeathScreen buttons")
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::Stats(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if screen.take_action() == Some(StatisticsAction::Done) {
                    self.active_screen = self.menu_parent_stack.pop().or_else(|| {
                        Some(ActiveScreen::Pause(PauseMenuScreen::new(
                            true,
                            self.has_singleplayer_server,
                            self.is_local_server,
                            std::sync::Arc::clone(&self.font)
                                as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                        )))
                    });
                    keep = false;
                }
                handled
            }
            ActiveScreen::Advancements(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                self.collect_seen_advancement_actions(screen);
                if screen.take_action()
                    == Some(el_gui::screens::advancements_screen::AdvancementsScreenAction::Done)
                {
                    screen.removed();
                    self.collect_seen_advancement_actions(screen);
                    self.active_screen = self.menu_parent_stack.pop().or_else(|| {
                        Some(ActiveScreen::Pause(PauseMenuScreen::new(
                            true,
                            self.has_singleplayer_server,
                            self.is_local_server,
                            std::sync::Arc::clone(&self.font)
                                as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                        )))
                    });
                    keep = false;
                }
                handled
            }
            ActiveScreen::AccessibilityOptions(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if screen.take_value_changed() {
                    self.apply_accessibility_options_settings(screen);
                    self.save_options();
                }
                if let Some(action) = screen.take_action() {
                    match action {
                        AccessibilityOptionsAction::Done => {
                            self.apply_accessibility_options_settings(screen);
                            if screen.parent_is_title {
                                self.open_title_screen();
                            } else {
                                self.active_screen =
                                    Some(self.menu_parent_or_options(screen.in_world));
                            }
                            self.save_options();
                            keep = false;
                        }
                        AccessibilityOptionsAction::AccessibilityLinkClicked => {
                            let _ = std::process::Command::new("xdg-open")
                                .arg("https://aka.ms/MinecraftJavaAccessibility")
                                .spawn();
                        }
                        AccessibilityOptionsAction::ControlsClicked => {
                            let child =
                                ActiveScreen::Controls(self.controls_screen(screen.in_world));
                            self.stash_current_menu_parent(&mut scr);
                            self.active_screen = Some(child);
                            keep = false;
                        }
                    }
                }
                handled
            }
            ActiveScreen::MouseSettings(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if screen.take_action().is_some() {
                    self.apply_mouse_settings(screen);
                    let in_world = screen.in_world();
                    self.active_screen = Some(self.pop_menu_parent_or(ActiveScreen::Controls(
                        self.controls_screen(in_world),
                    )));
                    self.save_options();
                    keep = false;
                }
                handled
            }
            ActiveScreen::Restrictions(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                match screen.take_action() {
                    Some(RestrictionsAction::Done) => {
                        self.active_screen =
                            Some(self.pop_menu_parent_or(ActiveScreen::WorldOptions(
                                self.world_options_screen(screen.in_world()),
                            )));
                        keep = false;
                    }
                    Some(RestrictionsAction::Resolve(
                        GuiChatRestrictionAction::GoToChatSettings,
                    )) => {
                        let child = ActiveScreen::ChatOptions(ChatOptionsScreen::new(
                            screen.in_world(),
                            &self.extra_options,
                            std::sync::Arc::clone(&self.font)
                                as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                        ));
                        self.stash_current_menu_parent(&mut scr);
                        self.active_screen = Some(child);
                        keep = false;
                    }
                    Some(RestrictionsAction::Resolve(GuiChatRestrictionAction::GoToAccount)) => {
                        let in_world = screen.in_world();
                        self.stash_current_menu_parent(&mut scr);
                        self.active_screen =
                            Some(ActiveScreen::ConfirmLink(ConfirmLinkScreen::new(
                                in_world,
                                "https://aka.ms/JavaAccountSettings".to_string(),
                                true,
                                std::sync::Arc::clone(&self.font)
                                    as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                            )));
                        keep = false;
                    }
                    None => {}
                }
                handled
            }
            ActiveScreen::LanguageSelect(screen) => {
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = screen.take_action() {
                    match action {
                        LanguageSelectAction::Done => {
                            let language_changed = self
                                .extra_options
                                .get("lang")
                                .map(String::as_str)
                                .unwrap_or("en_us")
                                != screen.selected_code;
                            self.extra_options
                                .insert("lang".to_string(), screen.selected_code.clone());
                            self.save_options();
                            crate::ui::i18n::reload_from_resource_manager(
                                &self.resource_manager,
                                &screen.selected_code,
                            );
                            self.resource_reload_pending |= language_changed;
                            if screen.parent_is_title {
                                self.open_title_screen();
                            } else {
                                self.active_screen =
                                    Some(self.menu_parent_or_options(screen.in_world));
                            }
                            keep = false;
                        }
                        LanguageSelectAction::FontSettingsClicked => {
                            let child = ActiveScreen::FontSettings(FontSettingsScreen::new(
                                screen.in_world,
                                self.font_options().uniform,
                                self.font_options().japanese_variants,
                            ));
                            self.stash_current_menu_parent(&mut scr);
                            self.active_screen = Some(child);
                            keep = false;
                        }
                    }
                }
                handled
            }
            ActiveScreen::FontSettings(screen) => {
                if pressed_key(event, KeyCode::Escape) {
                    self.active_screen =
                        Some(self.pop_menu_parent_or(ActiveScreen::LanguageSelect(
                            self.create_language_select_screen(screen.in_world),
                        )));
                    keep = false;
                    true
                } else if let Some((key_event, state)) = screen_events::key_event(
                    event,
                    screen_events::HeldModifiers {
                        shift: self.shift_held,
                        control: self.control_held,
                    },
                    false,
                ) {
                    if state == ElementState::Pressed {
                        let previous_options = (screen.force_unicode, screen.japanese_glyphs);
                        let (handled, action) = screen.key_pressed(key_event);
                        if screen.force_unicode != previous_options.0 {
                            self.set_font_option("forceUnicodeFont", screen.force_unicode);
                        }
                        if screen.japanese_glyphs != previous_options.1 {
                            self.set_font_option("japaneseGlyphVariants", screen.japanese_glyphs);
                        }
                        if action == Some(FontSettingsAction::Done) {
                            self.active_screen =
                                Some(self.pop_menu_parent_or(ActiveScreen::LanguageSelect(
                                    self.create_language_select_screen(screen.in_world),
                                )));
                            keep = false;
                        }
                        handled || action.is_some()
                    } else {
                        true
                    }
                } else {
                    let scale =
                        self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
                    let logical_w = self.last_screen_w / scale;
                    let logical_h = self.last_screen_h / scale;
                    if let WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } = event
                    {
                        let previous_options = (screen.force_unicode, screen.japanese_glyphs);
                        if let Some(action) = screen.mouse_clicked(
                            self.mouse_x,
                            self.mouse_y,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        ) {
                            match action {
                                FontSettingsAction::Done => {
                                    self.active_screen = Some(self.pop_menu_parent_or(
                                        ActiveScreen::LanguageSelect(
                                            self.create_language_select_screen(screen.in_world),
                                        ),
                                    ));
                                }
                            }
                            keep = false;
                        }
                        if screen.force_unicode != previous_options.0 {
                            self.set_font_option("forceUnicodeFont", screen.force_unicode);
                        }
                        if screen.japanese_glyphs != previous_options.1 {
                            self.set_font_option("japaneseGlyphVariants", screen.japanese_glyphs);
                        }
                        true
                    } else {
                        true
                    }
                }
            }
            ActiveScreen::KeyBinds {
                screen,
                waiting_for_key,
            } => {
                if waiting_for_key.is_some() {
                    let bind_name = waiting_for_key.as_ref().unwrap().clone();
                    if controls.get(&bind_name).is_none() {
                        *waiting_for_key = None;
                        screen.replace_entries(build_keybind_entries(controls));
                        return true;
                    }
                    if let WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button,
                        ..
                    } = event
                    {
                        if let Some(btn_id) = mouse_button_id(*button) {
                            let new_input = crate::input::BoundInput::Mouse(btn_id);
                            if let Some(m) = controls.get_mut(&bind_name) {
                                m.set_input(new_input);
                                self.extra_options.insert(
                                    format!("key_{}", m.name),
                                    crate::input::key_mapping::serialize_bound_input(new_input),
                                );
                            }
                            self.save_options();
                            screen.replace_entries(build_keybind_entries(controls));
                            *waiting_for_key = None;
                        }
                        return true;
                    }
                    if let WindowEvent::KeyboardInput {
                        event:
                            winit::event::KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(code),
                                repeat: false,
                                ..
                            },
                        ..
                    } = event
                    {
                        let new_input = if *code == KeyCode::Escape {
                            crate::input::BoundInput::Unbound
                        } else {
                            crate::input::BoundInput::Key(*code)
                        };
                        if let Some(m) = controls.get_mut(&bind_name) {
                            m.set_input(new_input);
                            self.extra_options.insert(
                                format!("key_{}", m.name),
                                crate::input::key_mapping::serialize_bound_input(new_input),
                            );
                        }
                        self.save_options();
                        screen.replace_entries(build_keybind_entries(controls));
                        *waiting_for_key = None;
                        return true;
                    }
                    return true;
                }
                let handled = route_migrated_screen(
                    screen,
                    event,
                    self.mouse_x,
                    self.mouse_y,
                    self.shift_held,
                    self.control_held,
                );
                if let Some(action) = screen.take_action() {
                    match action {
                        KeyBindsAction::Done => {
                            self.active_screen = Some(self.pop_menu_parent_or(
                                ActiveScreen::Controls(self.controls_screen(screen.in_world)),
                            ));
                            keep = false;
                        }
                        KeyBindsAction::ResetAll => {
                            for m in controls.mappings_mut() {
                                m.set_input(m.default_input);
                                self.extra_options
                                    .insert(format!("key_{}", m.name), m.save_string());
                            }
                            self.save_options();
                            screen.replace_entries(build_keybind_entries(controls));
                            keep = true;
                        }
                        KeyBindsAction::SelectKey { id } => {
                            if let Some(e) = screen.entries.iter_mut().find(|e| e.id == id) {
                                *waiting_for_key = Some(e.name_key.clone());
                                if !e.binding_name.starts_with("> ") {
                                    e.binding_name = format!("> {} <", e.binding_name);
                                }
                            }
                            keep = true;
                        }
                        KeyBindsAction::ResetKey { id } => {
                            if let Some(m) = controls.mappings_mut().get_mut(id) {
                                m.set_input(m.default_input);
                                self.extra_options
                                    .insert(format!("key_{}", m.name), m.save_string());
                            }
                            self.save_options();
                            screen.replace_entries(build_keybind_entries(controls));
                            keep = true;
                        }
                    }
                }
                handled
            }
            ActiveScreen::SubmenuStub { submenu, in_world } => {
                let is_options_submenu = match submenu {
                    PauseSubmenu::Singleplayer
                    | PauseSubmenu::Advancements
                    | PauseSubmenu::Stats
                    | PauseSubmenu::ReportBugs
                    | PauseSubmenu::Feedback
                    | PauseSubmenu::Friends
                    | PauseSubmenu::Realms
                    | PauseSubmenu::PlayerReporting
                    | PauseSubmenu::MultiplayerOptions
                    | PauseSubmenu::Credits
                    | PauseSubmenu::DemoReset => false,
                    _ => true,
                };
                if pressed_key(event, KeyCode::Escape) {
                    self.close_submenu_stub(*in_world, is_options_submenu);
                    keep = false;
                    true
                } else if pressed_key(event, KeyCode::Enter)
                    || pressed_key(event, KeyCode::NumpadEnter)
                    || pressed_key(event, KeyCode::Space)
                {
                    el_gui::components::click_sound_sink::record_button_click();
                    self.close_submenu_stub(*in_world, is_options_submenu);
                    keep = false;
                    true
                } else if let WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } = event
                {
                    let scale =
                        self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
                    let logical_w = self.last_screen_w / scale;
                    let logical_h = self.last_screen_h / scale;
                    let button_w = 200.0;
                    let button_h = 20.0;
                    let x = (logical_w - button_w) * 0.5;
                    let y = logical_h - 28.0;
                    if self.mouse_x >= x
                        && self.mouse_x < x + button_w
                        && self.mouse_y >= y
                        && self.mouse_y < y + button_h
                    {
                        el_gui::components::click_sound_sink::record_button_click();
                        self.close_submenu_stub(*in_world, is_options_submenu);
                        keep = false;
                    }
                    true
                } else {
                    true
                }
            }
            ActiveScreen::Chat { input } => {
                self.handle_chat_screen_input(input, false, event, player, &mut keep)
            }
            ActiveScreen::InBedChat(screen) => {
                let scale =
                    self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
                let logical_w_i = (self.last_screen_w / scale).round() as i32;
                let logical_h_i = (self.last_screen_h / scale).round() as i32;
                if screen.base_ref().width != logical_w_i || screen.base_ref().height != logical_h_i
                {
                    el_gui::framework::screen::screen_init(screen, logical_w_i, logical_h_i);
                }
                let mut leave_bed_requested = false;
                if let WindowEvent::MouseInput {
                    state: ElementState::Pressed,
                    button: MouseButton::Left,
                    ..
                } = event
                {
                    let click_event = el_gui::input::events::MouseButtonEvent::new(
                        self.mouse_x as f64,
                        self.mouse_y as f64,
                        el_gui::input::events::MouseButtonInfo::new(0, 0),
                    );
                    let clicked = Screen::mouse_clicked(screen, click_event, false);
                    if clicked || screen.take_action() == Some(InBedChatAction::StopSleeping) {
                        leave_bed_requested = true;
                    }
                }
                if leave_bed_requested {
                    self.pending_unattended_chat_commands
                        .push("/stop_sleeping".to_string());
                    true
                } else {
                    self.handle_chat_screen_input(&mut screen.input, true, event, player, &mut keep)
                }
            }
        };
        if let Some((url, return_to_dialog)) = pending_server_dialog_link_prompt.take() {
            let in_world = scr.is_in_world();
            if return_to_dialog {
                self.stash_current_menu_parent(&mut scr);
            } else if let ActiveScreen::ServerDialog { parent, .. } = &mut scr
                && let Some(previous) = parent.take()
            {
                self.menu_parent_stack.push(*previous);
            }
            self.active_screen = Some(ActiveScreen::ConfirmLink(ConfirmLinkScreen::new(
                in_world,
                url,
                false,
                std::sync::Arc::clone(&self.font) as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
            )));
            keep = false;
        }
        if handled {
            if let Some(click) = click_to_record {
                self.last_migrated_screen_click = Some(click);
            }
        }
        if keep {
            if let ActiveScreen::Video(video_screen) = &scr {
                self.apply_video_settings(video_screen);
            }
        }
        if keep && !matches!(scr, ActiveScreen::NavigationTransition) {
            self.active_screen = Some(scr);
        }
        handled
    }
    fn handle_chat_screen_input(
        &mut self,
        input: &mut String,
        is_in_bed: bool,
        event: &WindowEvent,
        player: &mut LocalPlayer,
        keep: &mut bool,
    ) -> bool {
        if pressed_key(event, KeyCode::Escape) {
            if self.chat_suggestions_popup_open {
                self.chat_suggestions_popup_open = false;
                self.active_chat_suggestions.clear();
                self.chat_suggestion_original_contents = None;
                self.chat_tab_cycles = false;
                return true;
            }
            if is_in_bed {
                self.pending_unattended_chat_commands
                    .push("/stop_sleeping".to_string());
                return true;
            }
            self.server_chat_suggestions.clear();
            self.server_suggestion_input = None;
            self.chat_suggestion_index = 0;
            self.chat_tab_cycles = false;
            self.chat_suggestions_allowed = false;
            self.chat_suggestions_popup_open = false;
            self.active_chat_suggestions.clear();
            self.chat_suggestion_original_contents = None;
            self.chat_removed_draft_rules(input);
            self.chat_component.reset_chat_scroll();
            self.active_screen = None;
            *keep = false;
            true
        } else if pressed_key(event, KeyCode::Enter) || pressed_key(event, KeyCode::NumpadEnter) {
            let message = screen::normalize_chat_message(input);
            let allowed = if message.starts_with('/') {
                self.chat_visibility_allows_system()
            } else {
                self.chat_visibility_allows_player()
            };
            self.chat_component.discard_draft();
            self.chat_component.reset_chat_scroll();
            let is_in_bed = is_in_bed;
            if is_in_bed {
                input.clear();
                self.chat_cursor = 0;
                self.chat_selection_anchor = None;
                self.chat_input_display_pos = 0;
                self.server_chat_suggestions.clear();
                self.server_suggestion_input = None;
                self.chat_suggestion_index = 0;
                self.chat_tab_cycles = false;
                self.chat_suggestions_popup_open = false;
                self.active_chat_suggestions.clear();
                self.chat_suggestion_original_contents = None;
                self.chat_is_draft = false;
            } else {
                self.active_screen = None;
                *keep = false;
            }
            if allowed && !message.is_empty() {
                self.submit_chat(message, player);
            }
            true
        } else if self.control_held && pressed_key(event, KeyCode::KeyA) {
            self.chat_selection_anchor = Some(0);
            self.chat_cursor = input.chars().count();
            self.chat_tab_cycles = false;
            self.active_chat_suggestions.clear();
            self.chat_suggestion_original_contents = None;
            true
        } else if self.control_held && pressed_key(event, KeyCode::KeyC) {
            if let Some((start, end)) =
                chat_selection_range(self.chat_selection_anchor, self.chat_cursor)
            {
                let selected = chat_text_range(input, start, end).to_string();
                self.chat_clipboard = selected.clone();
                el_gui::components::edit_box::Clipboard::set(
                    &mut *self.clipboard.borrow_mut(),
                    selected,
                );
            }
            self.chat_tab_cycles = false;
            true
        } else if self.control_held && pressed_key(event, KeyCode::KeyX) {
            if let Some((start, end)) =
                chat_selection_range(self.chat_selection_anchor, self.chat_cursor)
            {
                let selected = chat_text_range(input, start, end).to_string();
                self.chat_clipboard = selected.clone();
                el_gui::components::edit_box::Clipboard::set(
                    &mut *self.clipboard.borrow_mut(),
                    selected,
                );
                remove_chat_selection(
                    input,
                    &mut self.chat_cursor,
                    &mut self.chat_selection_anchor,
                );
                self.chat_suggestions_allowed = true;
                self.chat_suggestions_popup_open = input.starts_with('/');
                self.chat_is_draft = false;
                self.chat_tab_cycles = false;
                self.active_chat_suggestions.clear();
                self.chat_suggestion_original_contents = None;
                self.queue_chat_suggestion_request(input);
            }
            true
        } else if self.control_held && pressed_key(event, KeyCode::KeyV) {
            remove_chat_selection(
                input,
                &mut self.chat_cursor,
                &mut self.chat_selection_anchor,
            );
            self.chat_suggestions_allowed = true;
            self.chat_suggestions_popup_open = input.starts_with('/');
            let clip_text = el_gui::components::edit_box::Clipboard::get(&*self.clipboard.borrow());
            let clip_ref = if clip_text.is_empty() {
                &self.chat_clipboard
            } else {
                &clip_text
            };
            let remaining = 256usize.saturating_sub(screen::utf16_len(input));
            let pasted_source = clip_ref
                .chars()
                .filter(|&ch| el_gui::components::edit_box::is_allowed_chat_character(ch))
                .collect::<String>();
            let pasted = screen::truncate_utf16(&pasted_source, remaining);
            insert_text_at(input, self.chat_cursor, &pasted);
            self.chat_cursor += pasted.chars().count();
            self.chat_is_draft = false;
            self.chat_suggestion_index = 0;
            self.chat_tab_cycles = false;
            self.active_chat_suggestions.clear();
            self.chat_suggestion_original_contents = None;
            self.queue_chat_suggestion_request(input);
            true
        } else if pressed_key(event, KeyCode::Tab) {
            self.chat_suggestions_allowed = true;
            if !self.chat_suggestions_popup_open || self.active_chat_suggestions.is_empty() {
                let suggestions = self.chat_suggestions_for(input);
                if !suggestions.is_empty() {
                    self.chat_suggestions_popup_open = true;
                    self.chat_suggestion_original_contents = Some(input.clone());
                    self.active_chat_suggestions = suggestions;
                    self.chat_suggestion_index = 0;
                    self.chat_tab_cycles = false;
                    self.chat_suggestion_offset = 0;
                } else {
                    self.queue_chat_suggestion_request(input);
                }
            } else {
                if !self.chat_tab_cycles {
                    self.chat_tab_cycles = true;
                } else {
                    let len = self.active_chat_suggestions.len();
                    if self.shift_held {
                        self.chat_suggestion_index = self
                            .chat_suggestion_index
                            .checked_sub(1)
                            .unwrap_or(len.saturating_sub(1));
                    } else {
                        self.chat_suggestion_index = (self.chat_suggestion_index + 1) % len;
                    }
                }
                if let Some(suggestion) = self
                    .active_chat_suggestions
                    .get(self.chat_suggestion_index % self.active_chat_suggestions.len())
                {
                    let base = self
                        .chat_suggestion_original_contents
                        .as_deref()
                        .unwrap_or(input);
                    *input = screen::truncate_utf16(&suggestion.apply(base), 256);
                    self.chat_cursor = suggestion.replacement_end();
                    self.chat_selection_anchor = None;
                    self.chat_is_draft = false;
                    self.chat_suggestion_offset = suggestion_offset_for_selection(
                        self.chat_suggestion_index,
                        self.active_chat_suggestions.len(),
                        self.chat_suggestion_offset,
                    );
                }
            }
            true
        } else if pressed_key(event, KeyCode::ArrowUp) || pressed_key(event, KeyCode::ArrowDown) {
            if self.chat_suggestions_popup_open && !self.active_chat_suggestions.is_empty() {
                let len = self.active_chat_suggestions.len();
                if pressed_key(event, KeyCode::ArrowUp) {
                    self.chat_suggestion_index = self
                        .chat_suggestion_index
                        .checked_sub(1)
                        .unwrap_or(len.saturating_sub(1));
                } else {
                    self.chat_suggestion_index = (self.chat_suggestion_index + 1) % len;
                }
                self.chat_suggestion_offset = suggestion_offset_for_selection(
                    self.chat_suggestion_index,
                    len,
                    self.chat_suggestion_offset,
                );
                self.chat_tab_cycles = false;
                return true;
            }
            let history = self.chat_component.recent_chat();
            let max = history.len();
            let next = if pressed_key(event, KeyCode::ArrowUp) {
                self.chat_history_pos.saturating_sub(1)
            } else {
                (self.chat_history_pos + 1).min(max)
            };
            if next != self.chat_history_pos {
                if self.chat_history_pos == max {
                    self.chat_history_buffer = input.clone();
                }
                *input = if next == max {
                    self.chat_history_buffer.clone()
                } else {
                    history[next].clone()
                };
                self.chat_history_pos = next;
                self.chat_cursor = input.chars().count();
                self.chat_selection_anchor = None;
                self.chat_suggestion_index = 0;
                self.chat_tab_cycles = false;
                self.chat_is_draft = false;
                self.chat_suggestions_allowed = next == max;
                self.chat_suggestions_popup_open = false;
            }
            true
        } else if pressed_key(event, KeyCode::PageUp) || pressed_key(event, KeyCode::PageDown) {
            let options = self.chat_render_options();
            let page = self
                .chat_component
                .lines_per_page(options, true)
                .saturating_sub(1) as i32;
            let amount = if pressed_key(event, KeyCode::PageUp) {
                page
            } else {
                -page
            };
            self.chat_component
                .scroll_chat(amount, &self.font, options, true);
            self.chat_tab_cycles = false;
            true
        } else if let WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Left,
            ..
        } = event
        {
            let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
            let logical_w = self.last_screen_w / scale;
            let logical_h = self.last_screen_h / scale;
            let mut click_consumed = false;
            if self.chat_component.queue_hit_at(
                &self.font,
                logical_h,
                self.chat_render_options(),
                self.mouse_x,
                self.mouse_y,
            ) {
                self.chat_component
                    .accept_next_delayed_message(self.gui_ticks);
                chat_parity_probe("phase=ui_hit_test result=skipped reason=delayed_queue");
                click_consumed = true;
            }
            if !click_consumed && !self.chat_visibility_allows_player() {
                let options = self.chat_render_options();
                if self.chat_component.restricted_prompt_at(
                    &self.font,
                    logical_h,
                    options,
                    self.mouse_x,
                    self.mouse_y,
                ) {
                    if self.option_bool("saveChatDrafts", false)
                        && !self.chat_is_draft
                        && !input.trim().is_empty()
                    {
                        self.chat_component.save_as_draft(input.clone());
                    } else {
                        self.chat_component.discard_draft();
                    }
                    self.active_screen = Some(ActiveScreen::Restrictions(
                        self.chat_restrictions_screen(true),
                    ));
                    chat_parity_probe("phase=ui_hit_test result=skipped reason=restricted_prompt");
                    click_consumed = true;
                }
            }
            let suggestions =
                if self.chat_suggestions_popup_open && !self.active_chat_suggestions.is_empty() {
                    self.active_chat_suggestions.clone()
                } else {
                    self.chat_suggestions_for(input)
                };
            let suggestion_limit = suggestions.len().min(10);
            let max_suggestion_offset = suggestions.len().saturating_sub(10);
            self.chat_suggestion_offset = self.chat_suggestion_offset.min(max_suggestion_offset);
            let visible_suggestions = suggestions
                .iter()
                .enumerate()
                .skip(self.chat_suggestion_offset)
                .take(10)
                .collect::<Vec<_>>();
            if !click_consumed && self.chat_suggestions_popup_open {
                let suggestion_width = suggestions
                    .iter()
                    .map(|suggestion| self.font.measure_width(&suggestion.display, 1.0))
                    .map(f32::ceil)
                    .fold(0.0_f32, f32::max);
                let suggestion_x = chat_suggestion_x(
                    &self.font,
                    input,
                    suggestions
                        .first()
                        .map(|suggestion| suggestion.range_start)
                        .unwrap_or(0),
                    self.chat_input_display_pos,
                    suggestion_width,
                    logical_w,
                );
                let suggestion_box_width = suggestion_width + 1.0;
                let suggestion_y = logical_h - 15.0 - suggestion_limit as f32 * 12.0;
                for (index, suggestion) in visible_suggestions.iter() {
                    let row_y = suggestion_y + (index - self.chat_suggestion_offset) as f32 * 12.0;
                    if self.mouse_x > suggestion_x
                        && self.mouse_x < suggestion_x + suggestion_box_width
                        && self.mouse_y > row_y
                        && self.mouse_y < row_y + 12.0
                    {
                        let base = self
                            .chat_suggestion_original_contents
                            .as_deref()
                            .unwrap_or(input);
                        *input = screen::truncate_utf16(&suggestion.apply(base), 256);
                        self.chat_cursor = suggestion.replacement_end();
                        self.chat_selection_anchor = None;
                        self.chat_suggestion_index = 0;
                        self.chat_suggestion_offset = 0;
                        self.chat_tab_cycles = false;
                        self.active_chat_suggestions.clear();
                        self.chat_suggestion_original_contents = None;
                        self.chat_suggestions_popup_open = false;
                        self.chat_is_draft = false;
                        self.queue_chat_suggestion_request(input);
                        chat_parity_probe("phase=ui_hit_test result=skipped reason=suggestion");
                        click_consumed = true;
                        break;
                    }
                }
            }
            if !click_consumed {
                let options = self.chat_render_options();
                if let Some(interaction) = self.chat_component.click_interaction_at(
                    &self.font,
                    logical_h,
                    options,
                    self.mouse_x,
                    self.mouse_y,
                ) {
                    let action_kind = interaction
                        .action
                        .as_ref()
                        .map(chat_click_action_name)
                        .unwrap_or("none");
                    let insertion_present = interaction.insertion.is_some();
                    chat_parity_probe(format!(
                        "phase=ui_hit_test result=hit action={action_kind} insertion_present={insertion_present} shift={} connected={}",
                        self.shift_held, self.connected_chat,
                    ));
                    if self.shift_held {
                        if let Some(insertion) = interaction.insertion {
                            remove_chat_selection(
                                input,
                                &mut self.chat_cursor,
                                &mut self.chat_selection_anchor,
                            );
                            self.chat_suggestions_allowed = true;
                            let remaining = 256usize.saturating_sub(screen::utf16_len(input));
                            let inserted_source = insertion
                                .chars()
                                .filter(|&ch| {
                                    el_gui::components::edit_box::is_allowed_chat_character(ch)
                                })
                                .collect::<String>();
                            let inserted = screen::truncate_utf16(&inserted_source, remaining);
                            insert_text_at(input, self.chat_cursor, &inserted);
                            self.chat_cursor += inserted.chars().count();
                            self.chat_is_draft = false;
                            self.chat_suggestion_index = 0;
                            self.chat_tab_cycles = false;
                            self.queue_chat_suggestion_request(input);
                            click_consumed = true;
                        }
                    } else if let Some(action) = interaction.action {
                        match action {
                            ChatClickAction::RunCommand { command } => {
                                if self.connected_chat {
                                    self.pending_unattended_chat_commands.push(
                                        command.strip_prefix('/').unwrap_or(&command).to_string(),
                                    );
                                }
                            }
                            ChatClickAction::Custom { id, payload } => {
                                let (payload_kind, payload_bytes) = match payload.as_ref() {
                                    Some(payload) => ("opaque_nbt", payload.len()),
                                    None => ("absent_tag_end", 0),
                                };
                                if self.connected_chat {
                                    self.pending_custom_click_actions.push((id, payload));
                                    chat_parity_probe(format!(
                                        "phase=ui_custom_enqueue result=queued payload_kind={payload_kind} payload_bytes={payload_bytes}",
                                    ));
                                } else {
                                    chat_parity_probe(format!(
                                        "phase=ui_custom_enqueue result=dropped_not_connected payload_kind={payload_kind} payload_bytes={payload_bytes}",
                                    ));
                                }
                            }
                            ChatClickAction::SuggestCommand { command } => {
                                *input = screen::truncate_utf16(&command, 256);
                                self.chat_cursor = input.chars().count();
                                self.chat_selection_anchor = None;
                                self.chat_is_draft = false;
                                self.chat_suggestions_allowed = true;
                                self.chat_suggestion_index = 0;
                                self.chat_tab_cycles = false;
                                self.queue_chat_suggestion_request(input);
                            }
                            ChatClickAction::CopyToClipboard { value } => {
                                self.chat_clipboard = value.clone();
                                el_gui::components::edit_box::Clipboard::set(
                                    &mut *self.clipboard.borrow_mut(),
                                    value,
                                );
                            }
                            ChatClickAction::OpenUrl { url } => {
                                if is_allowed_untrusted_chat_url(&url)
                                    && self.option_bool("chatLinks", true)
                                {
                                    if self.option_bool("chatLinksPrompt", true) {
                                        self.pending_link_return_input = Some(input.clone());
                                        let in_world = self.rendering_in_world;
                                        self.active_screen = Some(ActiveScreen::ConfirmLink(
                                            ConfirmLinkScreen::new(
                                                in_world,
                                                url,
                                                false,
                                                std::sync::Arc::clone(&self.font)
                                                    as std::sync::Arc<
                                                        dyn el_gui::FontWidthMeasurer,
                                                    >,
                                            ),
                                        ));
                                        *keep = false;
                                    } else {
                                        open_url_cross_platform(&url);
                                    }
                                } else if !is_allowed_untrusted_chat_url(&url) {
                                    log::warn!(
                                        "[Chat Security] Blocked untrusted URL with invalid protocol: {url}"
                                    );
                                }
                            }
                            ChatClickAction::OpenFile { path } => {
                                if interaction.source
                                    == crate::ui::chat::chat_component::GuiMessageSource::SystemClient
                                {
                                    open_file_cross_platform(&path);
                                } else {
                                    log::warn!(
                                        "[Chat Security] Blocked remote OpenFile click action for path: {path}"
                                    );
                                }
                            }
                            unsupported => {
                                log::warn!(
                                    "[Chat] Manual server click action is not yet supported: {}",
                                    chat_click_action_name(&unsupported)
                                );
                            }
                        }
                        click_consumed = true;
                    }
                } else {
                    chat_parity_probe(format!(
                        "phase=ui_hit_test result=miss shift={} connected={}",
                        self.shift_held, self.connected_chat,
                    ));
                }
            }
            if !click_consumed && self.mouse_y >= logical_h - 12.0 && self.mouse_y < logical_h {
                let old_cursor = self.chat_cursor;
                let input_width = (logical_w - 4.0).max(1.0);
                self.chat_input_display_pos = chat_input_scroll_to(
                    &self.font,
                    input,
                    self.chat_cursor,
                    self.chat_input_display_pos,
                    input_width,
                );
                let (display_start, display_end) = chat_input_display_range(
                    &self.font,
                    input,
                    self.chat_input_display_pos,
                    input_width,
                );
                let start_byte = byte_index_at_char(input, display_start);
                let end_byte = byte_index_at_char(input, display_end);
                let displayed_input = &input[start_byte..end_byte];
                let clicked = display_start
                    + chat_cursor_from_x(
                        &self.font,
                        displayed_input,
                        (self.mouse_x - 4.0).max(0.0),
                    );
                let now = web_time::Instant::now();
                let is_double_click = self
                    .last_chat_click_time
                    .map_or(false, |last| now.duration_since(last).as_millis() < 250);
                self.last_chat_click_time = Some(now);
                self.last_chat_click_cursor = clicked;
                if is_double_click {
                    let word_start = chat_word_position(input, -1, clicked);
                    let word_end = chat_word_position(input, 1, clicked);
                    self.chat_selection_anchor = Some(word_start);
                    self.chat_cursor = word_end;
                } else if self.shift_held {
                    self.chat_selection_anchor.get_or_insert(old_cursor);
                    self.chat_cursor = clicked;
                } else {
                    self.chat_cursor = clicked;
                    self.chat_selection_anchor = None;
                }
                self.chat_tab_cycles = false;
                self.queue_chat_suggestion_request(input);
            }
            #[cfg(target_os = "android")]
            {
                let tapped_input = self.mouse_y >= logical_h - 12.0 && self.mouse_y < logical_h;
                if !click_consumed && !tapped_input {
                    self.chat_component.discard_draft();
                    self.chat_component.reset_chat_scroll();
                    self.active_screen = None;
                    crate::hide_android_keyboard();
                }
            }
            true
        } else if pressed_key(event, KeyCode::ArrowLeft) {
            let old_cursor = self.chat_cursor;
            let target_pos = if self.control_held {
                chat_word_position(input, -1, self.chat_cursor)
            } else if !self.shift_held {
                if let Some((start, _)) =
                    chat_selection_range(self.chat_selection_anchor, self.chat_cursor)
                {
                    start
                } else {
                    self.chat_cursor.saturating_sub(1)
                }
            } else {
                self.chat_cursor.saturating_sub(1)
            };
            if !self.shift_held {
                self.chat_cursor = target_pos;
                self.chat_selection_anchor = None;
            } else {
                self.chat_selection_anchor.get_or_insert(old_cursor);
                self.chat_cursor = target_pos;
            }
            self.chat_tab_cycles = false;
            self.queue_chat_suggestion_request(input);
            true
        } else if pressed_key(event, KeyCode::ArrowRight) {
            let old_cursor = self.chat_cursor;
            let target_pos = if self.control_held {
                chat_word_position(input, 1, self.chat_cursor)
            } else if !self.shift_held {
                if let Some((_, end)) =
                    chat_selection_range(self.chat_selection_anchor, self.chat_cursor)
                {
                    end
                } else {
                    (self.chat_cursor + 1).min(input.chars().count())
                }
            } else {
                (self.chat_cursor + 1).min(input.chars().count())
            };
            if !self.shift_held {
                self.chat_cursor = target_pos;
                self.chat_selection_anchor = None;
            } else {
                self.chat_selection_anchor.get_or_insert(old_cursor);
                self.chat_cursor = target_pos;
            }
            self.chat_tab_cycles = false;
            self.queue_chat_suggestion_request(input);
            true
        } else if pressed_key(event, KeyCode::Home) {
            let old_cursor = self.chat_cursor;
            self.chat_cursor = 0;
            if self.shift_held {
                self.chat_selection_anchor.get_or_insert(old_cursor);
            } else {
                self.chat_selection_anchor = None;
            }
            self.chat_tab_cycles = false;
            self.queue_chat_suggestion_request(input);
            true
        } else if pressed_key(event, KeyCode::End) {
            let old_cursor = self.chat_cursor;
            self.chat_cursor = input.chars().count();
            if self.shift_held {
                self.chat_selection_anchor.get_or_insert(old_cursor);
            } else {
                self.chat_selection_anchor = None;
            }
            self.chat_tab_cycles = false;
            self.queue_chat_suggestion_request(input);
            true
        } else if pressed_key(event, KeyCode::Backspace) {
            if self.chat_is_draft {
                input.clear();
                self.chat_cursor = 0;
                self.chat_selection_anchor = None;
                self.chat_suggestions_allowed = true;
                self.chat_suggestions_popup_open = false;
                self.chat_is_draft = false;
            } else if remove_chat_selection(
                input,
                &mut self.chat_cursor,
                &mut self.chat_selection_anchor,
            ) {
            } else if self.control_held {
                delete_word_before(input, &mut self.chat_cursor);
            } else {
                remove_char_before(input, &mut self.chat_cursor);
            }
            self.chat_is_draft = false;
            self.chat_suggestion_index = 0;
            self.chat_tab_cycles = false;
            self.chat_suggestions_allowed = true;
            self.active_chat_suggestions.clear();
            self.chat_suggestions_popup_open = input.starts_with('/');
            self.chat_suggestion_original_contents = None;
            self.queue_chat_suggestion_request(input);
            true
        } else if pressed_key(event, KeyCode::Delete) {
            if !remove_chat_selection(
                input,
                &mut self.chat_cursor,
                &mut self.chat_selection_anchor,
            ) {
                if self.control_held {
                    delete_word_after(input, self.chat_cursor);
                } else {
                    remove_char_at(input, self.chat_cursor);
                }
            }
            self.chat_suggestions_allowed = true;
            self.active_chat_suggestions.clear();
            self.chat_suggestions_popup_open = input.starts_with('/');
            self.chat_is_draft = false;
            self.chat_suggestion_index = 0;
            self.chat_tab_cycles = false;
            self.chat_suggestion_original_contents = None;
            self.queue_chat_suggestion_request(input);
            true
        } else if let WindowEvent::KeyboardInput {
            event: key_event, ..
        } = event
        {
            if key_event.state == ElementState::Pressed {
                let text =
                    key_event.text.as_deref().map(str::to_owned).or_else(|| {
                        match &key_event.logical_key {
                            winit::keyboard::Key::Character(character) => {
                                Some(character.to_string())
                            }
                            _ => None,
                        }
                    });
                if let Some(text) = text {
                    remove_chat_selection(
                        input,
                        &mut self.chat_cursor,
                        &mut self.chat_selection_anchor,
                    );
                    for ch in text.chars() {
                        if el_gui::components::edit_box::is_allowed_chat_character(ch)
                            && screen::utf16_len(input).saturating_add(ch.len_utf16()) <= 256
                        {
                            insert_char_at(input, self.chat_cursor, ch);
                            self.chat_cursor += 1;
                        }
                    }
                    self.chat_suggestions_allowed = true;
                    self.active_chat_suggestions.clear();
                    self.chat_suggestions_popup_open = input.starts_with('/');
                    self.chat_is_draft = false;
                    self.chat_suggestion_index = 0;
                    self.chat_tab_cycles = false;
                    self.chat_suggestion_original_contents = None;
                    self.queue_chat_suggestion_request(input);
                }
            }
            true
        } else {
            true
        }
    }
    fn chat_removed_draft_rules(&mut self, input: &str) {
        let discards = !self.option_bool("saveChatDrafts", false) || input.trim().is_empty();
        if discards {
            self.chat_component.discard_draft();
        } else if !self.chat_is_draft {
            self.chat_component.save_as_draft(input.to_string());
        }
    }
    fn handle_pause_action(&mut self, action: PauseMenuAction) {
        match action {
            PauseMenuAction::ReturnToGame => self.active_screen = None,
            PauseMenuAction::OpenSubmenu(submenu) => {
                if submenu == PauseSubmenu::Options {
                    self.active_screen = Some(ActiveScreen::Options(OptionsScreen::new(
                        true,
                        std::sync::Arc::clone(&self.font)
                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                    )));
                } else {
                    self.active_screen = Some(ActiveScreen::SubmenuStub {
                        submenu,
                        in_world: true,
                    });
                }
            }
            PauseMenuAction::Disconnect => {
                self.requested_disconnect = true;
                self.active_screen = None;
            }
            PauseMenuAction::OpenExtensionCatalog(kind) => {
                self.open_extension_catalog(
                    kind,
                    ActiveScreen::Pause(PauseMenuScreen::new(
                        true,
                        self.has_singleplayer_server,
                        self.is_local_server,
                        std::sync::Arc::clone(&self.font)
                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                    )),
                );
            }
            PauseMenuAction::ConfirmFriendsList => {
                let parent = ActiveScreen::Pause(PauseMenuScreen::new(
                    true,
                    self.has_singleplayer_server,
                    self.is_local_server,
                    std::sync::Arc::clone(&self.font)
                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                ));
                let prompt = FriendsListConfirmScreen::new(
                    true,
                    std::sync::Arc::clone(&self.font)
                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                );
                self.active_screen = Some(ActiveScreen::FriendsConfirm {
                    screen: prompt,
                    parent: Some(Box::new(parent)),
                    in_world: true,
                });
            }
        }
    }
    fn handle_title_action(&mut self, action: TitleScreenAction) {
        match action {
            TitleScreenAction::PlayDemo => {
                self.active_screen = Some(ActiveScreen::SubmenuStub {
                    submenu: PauseSubmenu::Singleplayer,
                    in_world: false,
                });
            }
            TitleScreenAction::Multiplayer => {
                ui_crash_trace("Title stage=multiplayer_action");
                self.open_multiplayer_screen(false);
                ui_crash_trace("Title stage=multiplayer_action_done");
            }
            TitleScreenAction::Online => {
                self.active_screen = Some(ActiveScreen::SubmenuStub {
                    submenu: PauseSubmenu::Realms,
                    in_world: false,
                });
            }
            TitleScreenAction::Friends => {
                self.active_screen = Some(ActiveScreen::SubmenuStub {
                    submenu: PauseSubmenu::Friends,
                    in_world: false,
                });
            }
            TitleScreenAction::Language => {
                let mut screen = self.create_language_select_screen(false);
                screen.parent_is_title = true;
                self.active_screen = Some(ActiveScreen::LanguageSelect(screen));
            }
            TitleScreenAction::Accessibility => {
                let mut screen = AccessibilityOptionsScreen::new(
                    false,
                    &self.extra_options,
                    std::sync::Arc::clone(&self.font)
                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                );
                screen.parent_is_title = true;
                self.active_screen = Some(ActiveScreen::AccessibilityOptions(screen));
            }
            TitleScreenAction::Options => {
                self.active_screen = Some(ActiveScreen::Options(OptionsScreen::new(
                    false,
                    std::sync::Arc::clone(&self.font)
                        as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                )));
            }
            TitleScreenAction::CreditsAndAttribution => {
                self.active_screen = Some(ActiveScreen::CreditsAndAttribution(
                    CreditsAndAttributionScreen::new(
                        false,
                        true,
                        std::sync::Arc::clone(&self.font)
                            as std::sync::Arc<dyn el_gui::FontWidthMeasurer>,
                    ),
                ));
            }
            TitleScreenAction::Quit => {
                self.requested_quit = true;
                self.active_screen = None;
            }
            TitleScreenAction::ResetDemo => {
                self.active_screen = Some(ActiveScreen::SubmenuStub {
                    submenu: PauseSubmenu::DemoReset,
                    in_world: false,
                });
            }
            TitleScreenAction::OpenExtensionCatalog(_) => {}
        }
    }
    pub fn tick(
        &mut self,
        level: Option<&mut ClientLevel>,
        player: &mut LocalPlayer,
        active_hotbar_slot: usize,
        client_ticks: u32,
        show_death_screen: bool,
        hardcore: bool,
    ) {
        let has_level = level.is_some();
        let notification_display_time = self.option_float("notificationDisplayTime", 1.0);
        let message_delay_ticks =
            (self.option_float("chatDelay", 0.0).clamp(0.0, 6.0) * 20.0).round() as u32;
        if self.gui_ticks == 0 {
            self.gui_ticks = player.age_in_ticks.max(0.0) as u32;
        }
        for _ in 0..client_ticks {
            let bed_chat_open = matches!(self.active_screen, Some(ActiveScreen::InBedChat(_)));
            if bed_chat_open {
                if player.sleeping_position().is_none() {
                    self.on_player_woke_up();
                }
            } else if self.active_screen.is_some() {
            } else if has_level && player.is_dead_or_dying() {
                if show_death_screen {
                    self.open_death_screen(None, hardcore, None);
                } else {
                    self.pending_death_screen_respawn_request = true;
                }
            } else if has_level && player.sleeping_position().is_some() {
                self.open_in_bed_chat_screen();
            }
            self.gui_ticks = self.gui_ticks.wrapping_add(1);
            if self.title_time > 0 {
                self.title_time -= 1;
                if self.title_time == 0 {
                    self.server_title = None;
                    self.server_subtitle = None;
                }
            }
            self.hud
                .tick(player, active_hotbar_slot, notification_display_time, 1);
            self.chat_component
                .set_message_delay_ticks(message_delay_ticks, self.gui_ticks);
            self.chat_component.tick(self.gui_ticks);
            for message in self.hud.chat_messages.drain(..) {
                self.chat_component
                    .add_client_system_message(message, self.gui_ticks);
            }
            if let Some(ActiveScreen::LanguageSelect(screen)) = &mut self.active_screen {
                screen.tick();
            }
            if let Some(ActiveScreen::Confirm(screen)) = &mut self.active_screen {
                screen.tick();
            }
            if let Some(ActiveScreen::ServerReconfig(screen)) = &mut self.active_screen {
                screen.tick();
            }
            if let Some(ActiveScreen::ResourcePacks(screen)) = &mut self.active_screen {
                screen.tick();
            }
            if let Some(ActiveScreen::Title(screen)) = &mut self.active_screen {
                screen.tick();
            }
            if let Some(ActiveScreen::ServerDialog { screen, .. }) = &mut self.active_screen {
                screen.tick();
            }
            if let Some(ActiveScreen::Pause(screen)) = &mut self.active_screen {
                screen.tick();
            }
            if let Some(ActiveScreen::Multiplayer(screen)) = &mut self.active_screen {
                screen.tick();
            }
            if let Some(ActiveScreen::WinScreen { screen, .. }) = &mut self.active_screen {
                screen.tick();
            }
            if let Some(ActiveScreen::DeathScreen(screen)) = &mut self.active_screen {
                screen.tick();
            }
        }
        let win_screen_finished = match self.active_screen.as_mut() {
            Some(ActiveScreen::WinScreen { screen, .. }) => screen.take_action().is_some(),
            _ => false,
        };
        if win_screen_finished {
            if let Some(ActiveScreen::WinScreen { screen, parent }) = self.active_screen.take() {
                self.finish_win_screen(screen.poem(), parent);
            }
        }
        let mut lan_snapshot = None;
        if let Some(ref rx) = self.lan_servers_rx {
            while let Ok(servers) = rx.try_recv() {
                lan_snapshot = Some(servers);
            }
        }
        let mut ping_updates = Vec::new();
        if let Some(ref rx) = self.ping_results_rx {
            while let Ok((_, entry)) = rx.try_recv() {
                ping_updates.push(entry);
            }
        }
        let mut live_icons = Vec::new();
        {
            let target = match self.active_screen.as_mut() {
                Some(ActiveScreen::Multiplayer(screen)) => Some(screen),
                _ => self.stashed_multiplayer_screen.as_mut(),
            };
            if let Some(screen) = target {
                if let Some(servers) = lan_snapshot {
                    screen.set_lan_servers(servers);
                }
                for entry in ping_updates {
                    if let Some(pos) = screen.servers.iter().position(|s| s.ip == entry.ip) {
                        if let Some(bytes) = entry.icon_bytes.clone() {
                            live_icons.push((entry.ip.clone(), bytes));
                        }
                        screen.servers[pos] = entry;
                    }
                }
            }
        }
        for (server_ip, bytes) in live_icons {
            if !is_valid_server_favicon(&bytes) {
                self.update_server_icon(&server_ip, None);
                continue;
            }
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            server_ip.hash(&mut hasher);
            bytes.hash(&mut hasher);
            let key = hasher.finish();
            let icon_dir = self.game_directory().join("server_icons");
            let _ = std::fs::create_dir_all(&icon_dir);
            let path = icon_dir.join(format!("{key:016x}.png"));
            if std::fs::write(&path, &bytes).is_err() {
                continue;
            }
            let texture = ResourceLocation::new("eltharn", &format!("server_icons/{key:016x}"));
            self.resource_manager = self
                .resource_manager
                .clone()
                .with_external_images([(texture.clone(), path)]);
            self.update_server_icon(&server_ip, Some(texture));
            let mut list = crate::network::multiplayer::ServerList::new(self.game_directory());
            list.load();
            if let Some(server) = list.servers.iter_mut().find(|s| s.ip == server_ip) {
                server.icon_bytes = Some(bytes);
                list.save();
            }
        }
    }
    fn update_server_icon(&mut self, server_ip: &str, texture: Option<ResourceLocation>) {
        let clearing = texture.is_none();
        let target = match self.active_screen.as_mut() {
            Some(ActiveScreen::Multiplayer(screen)) => Some(screen),
            _ => self.stashed_multiplayer_screen.as_mut(),
        };
        if let Some(screen) = target {
            match texture {
                Some(texture) => screen.set_server_icon(server_ip.to_string(), texture),
                None => {
                    screen.remove_server_icon(server_ip);
                    if let Some(entry) = screen.servers.iter_mut().find(|s| s.ip == server_ip) {
                        entry.icon_bytes = None;
                    }
                }
            }
        }
        if clearing {
            let mut list = crate::network::multiplayer::ServerList::new(self.game_directory());
            list.load();
            if let Some(server) = list.servers.iter_mut().find(|s| s.ip == server_ip) {
                if server.icon_bytes.is_some() {
                    server.icon_bytes = None;
                    list.save();
                }
            }
        }
    }
    pub fn take_pending_container_clicks(&mut self) -> Vec<ServerboundContainerClickPacket> {
        std::mem::take(&mut self.pending_container_clicks)
    }
    pub fn take_pending_container_slot_state_changes(&mut self) -> Vec<(u32, i32, bool)> {
        std::mem::take(&mut self.pending_container_slot_state_changes)
    }
    pub fn take_pending_creative_clicks(&mut self) -> Vec<ServerboundSetCreativeModeSlotPacket> {
        std::mem::take(&mut self.pending_creative_clicks)
    }
    pub fn take_pending_bundle_item_selections(&mut self) -> Vec<(i32, u32)> {
        std::mem::take(&mut self.pending_bundle_item_selections)
    }
    pub fn take_requested_container_close(&mut self) -> Option<i32> {
        self.requested_container_close.take()
    }
    pub fn take_server_connection(
        &mut self,
    ) -> Option<(RemoteConnectionHandle, bool, ServerConnectionOrigin)> {
        let connection = self.server_connection.take()?;
        let is_transfer = std::mem::take(&mut self.server_connection_is_transfer);
        let origin = std::mem::take(&mut self.server_connection_origin);
        Some((connection, is_transfer, origin))
    }
    pub fn take_requested_game_mode(&mut self) -> Option<GameType> {
        self.requested_game_mode.take()
    }
    pub fn take_requested_difficulty(&mut self) -> Option<crate::world::client_level::Difficulty> {
        self.requested_difficulty.take()
    }
    pub fn take_requested_time(&mut self) -> Option<TimeRequest> {
        self.requested_time.take()
    }
    pub fn take_requested_weather(&mut self) -> Option<WeatherMode> {
        self.requested_weather.take()
    }
    pub fn take_requested_disconnect(&mut self) -> bool {
        let requested = self.requested_disconnect;
        self.requested_disconnect = false;
        requested
    }
    pub fn take_requested_quit(&mut self) -> bool {
        let requested = self.requested_quit;
        self.requested_quit = false;
        requested
    }
    fn commit_resource_pack_selection(&mut self, selected_entries: &[PackEntry]) {
        let selected_ids =
            crate::resources::pack_loader::selected_pack_ids_to_option_value(selected_entries);
        let packs_changed = crate::resources::pack_loader::resource_pack_selection_changed(
            self.extra_options
                .get("resourcePacks")
                .map(String::as_str)
                .unwrap_or_default(),
            selected_entries,
        );
        self.extra_options
            .insert("resourcePacks".to_string(), selected_ids);
        self.save_options();
        let pack_dir = self.resource_pack_directory();
        let extra_roots: Vec<std::path::PathBuf> = selected_entries
            .iter()
            .rev()
            .filter_map(|entry| {
                let rel = entry.id.strip_prefix("file/").unwrap_or(&entry.id);
                let path = pack_dir.join(rel);
                path.exists().then_some(path)
            })
            .collect();
        self.resource_manager = self.resource_manager.clone().with_extra_roots(extra_roots);
        self.resource_reload_pending |= packs_changed;
        log::info!(
            "[ResourcePacks] Applied {} packs to resource_manager (reload={})",
            selected_entries.len(),
            packs_changed
        );
    }
    pub fn inventory_entity_preview(&self) -> Option<InventoryEntityPreview> {
        let Some(ActiveScreen::Container(screen)) = self.active_screen.as_ref() else {
            return None;
        };
        Some(match screen.menu_type {
            el_inventory::menu::MenuType::Inventory => InventoryEntityPreview {
                x0: screen.left_pos as f32 + 26.0,
                y0: screen.top_pos as f32 + 8.0,
                x1: screen.left_pos as f32 + 75.0,
                y1: screen.top_pos as f32 + 78.0,
                size: 30.0,
                offset_y: 0.0625,
            },
            el_inventory::menu::MenuType::Creative => {
                if screen
                    .creative_state
                    .as_ref()
                    .map(|s| {
                        s.selected_kind() == el_inventory::creative::CreativeTabKind::Inventory
                    })
                    .unwrap_or(false)
                {
                    InventoryEntityPreview {
                        x0: screen.left_pos as f32 + 73.0,
                        y0: screen.top_pos as f32 + 6.0,
                        x1: screen.left_pos as f32 + 105.0,
                        y1: screen.top_pos as f32 + 49.0,
                        size: 20.0,
                        offset_y: 0.0625,
                    }
                } else {
                    return None;
                }
            }
            _ => return None,
        })
    }
    pub fn render_only_panorama<'rp>(
        &'rp mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        render_pass: &mut RenderPass<'rp>,
        screen_w: f32,
        screen_h: f32,
    ) {
        self.last_screen_w = screen_w;
        self.last_screen_h = screen_h;
        let scale = self.get_gui_scale_factor(screen_w, screen_h) as f32;
        let logical_w = screen_w / scale;
        let logical_h = screen_h / scale;
        let (glint_speed, glint_strength) = self.enchantment_glint_options();
        self.renderer.start_frame_with_glint(
            queue,
            logical_w,
            logical_h,
            glint_speed,
            glint_strength,
        );
        let realtime_delta_ticks = self.advance_realtime_delta_ticks();
        let spin = self.advance_menu_panorama(realtime_delta_ticks);
        self.render_title_panorama(device, queue, texture_bind_group_layout, spin);
        self.render_panorama_overlay(device, queue, texture_bind_group_layout);
        self.renderer.end_frame(device, queue);
        self.renderer.render(render_pass, &self.font);
    }
    fn render_server_title(&mut self, logical_w: f32, logical_h: f32) {
        let Some(title) = self.server_title.clone() else {
            return;
        };
        if self.title_time == 0 {
            return;
        }
        let fade_in = self.title_fade_in_time as f32;
        let stay = self.title_stay_time as f32;
        let fade_out = self.title_fade_out_time as f32;
        let time = self.title_time as f32;
        let total = fade_in + stay + fade_out;
        let mut alpha = 1.0f32;
        if time > fade_out + stay && fade_in > 0.0 {
            alpha = (total - time) / fade_in;
        }
        if time <= fade_out && fade_out > 0.0 {
            alpha = time / fade_out;
        }
        let alpha = alpha.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }
        let center_x = logical_w / 2.0;
        let center_y = logical_h / 2.0;
        let title_scale = 4.0;
        let title_width =
            crate::ui::chat::chat_component::formatted_chat_width(&self.font, &title, title_scale);
        let title_x = center_x - title_width / 2.0;
        let title_y = center_y - 40.0;
        self.render_title_backdrop(title_x, title_y, title_width, title_scale, alpha);
        crate::ui::chat::chat_component::draw_formatted_chat_text_ex(
            &mut self.renderer,
            &self.font,
            &title,
            title_x,
            title_y,
            alpha,
            title_scale,
            true,
        );
        if let Some(subtitle) = self.server_subtitle.clone() {
            let subtitle_scale = 2.0;
            let subtitle_width = crate::ui::chat::chat_component::formatted_chat_width(
                &self.font,
                &subtitle,
                subtitle_scale,
            );
            let subtitle_x = center_x - subtitle_width / 2.0;
            let subtitle_y = center_y + 10.0;
            self.render_title_backdrop(
                subtitle_x,
                subtitle_y,
                subtitle_width,
                subtitle_scale,
                alpha,
            );
            crate::ui::chat::chat_component::draw_formatted_chat_text_ex(
                &mut self.renderer,
                &self.font,
                &subtitle,
                subtitle_x,
                subtitle_y,
                alpha,
                subtitle_scale,
                true,
            );
        }
    }
    fn render_title_backdrop(&mut self, x: f32, y: f32, width: f32, scale: f32, alpha: f32) {
        let Some(backdrop) = self
            .hud_background_options()
            .text_with_backdrop_color(0.0, alpha)
        else {
            return;
        };
        self.renderer.draw_rect(
            x - 2.0 * scale,
            y - 2.0 * scale,
            width + 4.0 * scale,
            13.0 * scale,
            backdrop,
        );
    }
    pub(crate) fn prepare(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        player: &LocalPlayer,
        screen_w: f32,
        screen_h: f32,
        active_slot: usize,
        fps: u32,
        camera_type: CameraType,
        can_hurt_player: bool,
        hardcore: bool,
        has_experience: bool,
        prioritize_experience: bool,
        has_locator_waypoints: bool,
        locator_markers: &[LocatorBarMarker],
        crosshair_target_living: bool,
        defer_inventory_tooltip: bool,
        partial_tick: f32,
        profiler: Option<&mut RenderProfiler>,
    ) {
        let mut profiler = GuiPrepareProfiler::new(profiler);
        let prepare_started_at = profiler.started();
        self.last_screen_w = screen_w;
        self.last_screen_h = screen_h;
        let (rendering_in_world, rendering_title_screen) =
            screen_scene_flags(self.active_screen.as_ref());
        self.rendering_in_world = rendering_in_world;
        self.rendering_title_screen = rendering_title_screen;
        let scale = self.get_gui_scale_factor(screen_w, screen_h) as f32;
        let logical_w = screen_w / scale;
        let logical_h = screen_h / scale;
        self.update_logical_mouse();
        self.pending_cursor = el_gui::input::events::CursorType::Default;
        let skin_textures_started_at = profiler.started();
        self.renderer
            .poll_player_head_textures(device, queue, texture_bind_group_layout);
        let all_profile_skin_inputs = if self.online_mode {
            self.player_infos
                .values()
                .filter(|info| info.listed)
                .map(|info| (info.uuid, info.texture_url.clone()))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        self.renderer
            .set_player_profile_skin_urls(&all_profile_skin_inputs);
        let mut visible_tab_players = if self.show_player_list {
            self.player_infos
                .values()
                .filter(|info| info.listed)
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if !visible_tab_players.is_empty() {
            crate::ui::scoreboard::sort_tab_list_refs(&self.scoreboard, &mut visible_tab_players);
            visible_tab_players.truncate(80);
        }
        if self.online_mode {
            for info in visible_tab_players {
                let uuid = info.uuid;
                if let Some(url) = info.texture_url.as_deref() {
                    self.renderer.prepare_player_skin_texture(url);
                } else {
                    self.renderer.prepare_default_player_skin_texture(
                        device,
                        queue,
                        texture_bind_group_layout,
                        &self.resource_manager,
                        uuid,
                    );
                }
            }
        }
        for item in player.inventory.iter() {
            self.renderer.prepare_player_head_texture(item);
            self.renderer.prepare_player_head_default_texture(
                device,
                queue,
                texture_bind_group_layout,
                &self.resource_manager,
                item,
            );
        }
        if let Some(ActiveScreen::Container(screen)) = self.active_screen.as_ref() {
            for slot in 0..screen.menu.slots.len() {
                self.renderer
                    .prepare_player_head_texture(screen.menu.get_slot_item(slot));
                self.renderer.prepare_player_head_default_texture(
                    device,
                    queue,
                    texture_bind_group_layout,
                    &self.resource_manager,
                    screen.menu.get_slot_item(slot),
                );
            }
            self.renderer
                .prepare_player_head_texture(&screen.carried_item);
            self.renderer.prepare_player_head_default_texture(
                device,
                queue,
                texture_bind_group_layout,
                &self.resource_manager,
                &screen.carried_item,
            );
        }
        if let Some(ActiveScreen::Advancements(screen)) = self.active_screen.as_ref() {
            for display in screen
                .advancements()
                .iter()
                .filter_map(|node| node.display.as_ref())
            {
                if let Some(stack) = advancement_icon_stack(display) {
                    self.renderer.prepare_player_head_texture(&stack);
                    self.renderer.prepare_player_head_default_texture(
                        device,
                        queue,
                        texture_bind_group_layout,
                        &self.resource_manager,
                        &stack,
                    );
                }
            }
        }
        profiler.observe(
            "GuiManager::prepare::skin_textures",
            skin_textures_started_at,
        );
        let (glint_speed, glint_strength) = self.enchantment_glint_options();
        self.renderer.start_frame_with_glint(
            queue,
            logical_w,
            logical_h,
            glint_speed,
            glint_strength,
        );
        self.pending_chat_tooltip = None;
        self.pending_chat_hover = None;
        let active_screen_blocks_hud = self
            .active_screen
            .as_ref()
            .is_some_and(ActiveScreen::blocks_hud);
        let hud_started_at = profiler.started();
        let hud_visible = !self.hud_hidden && !active_screen_blocks_hud;
        #[cfg(feature = "extensions")]
        let hud_visible = crate::modding::hooks::hud_visible(hud_visible);
        if hud_visible {
            let attack_indicator = self.option_float("attackIndicator", 1.0).round() as i32;
            let attack_strength_scale = player.attack_strength_scale(0.0);
            let main_hand_left = self
                .extra_options
                .get("mainHand")
                .is_some_and(|value| value.eq_ignore_ascii_case("left"));
            let hud_background = self.hud_background_options();
            self.hud.render(
                &mut self.renderer,
                &self.font,
                player,
                logical_w,
                logical_h,
                active_slot,
                fps,
                camera_type,
                can_hurt_player,
                hardcore,
                has_experience,
                prioritize_experience,
                has_locator_waypoints,
                locator_markers,
                attack_indicator,
                attack_strength_scale,
                main_hand_left,
                crosshair_target_living && player.current_item_attack_strength_delay() > 5.0,
                hud_background,
            );
            #[cfg(target_os = "android")]
            if self.touch_controls.enabled && self.active_screen.is_none() {
                self.render_android_controls(logical_w, logical_h);
            }
            let hud_bg = self.hud_background_options();
            scoreboard_hud::render_scoreboard_sidebar(
                &mut self.renderer,
                &self.font,
                &self.scoreboard,
                &self.player_username,
                logical_w,
                logical_h,
                hud_bg,
            );
            self.render_server_title(logical_w, logical_h);
            if matches!(
                self.active_screen,
                Some(ActiveScreen::Chat { .. }) | Some(ActiveScreen::InBedChat(_))
            ) || self.chat_visibility_allows_system()
                || self.chat_visibility_allows_player()
            {
                let chat_mode = if matches!(
                    self.active_screen,
                    Some(ActiveScreen::Chat { .. }) | Some(ActiveScreen::InBedChat(_))
                ) {
                    if self.chat_visibility_allows_player() {
                        crate::ui::chat::ChatDisplayMode::Foreground
                    } else {
                        crate::ui::chat::ChatDisplayMode::ForegroundRestricted
                    }
                } else {
                    crate::ui::chat::ChatDisplayMode::Background
                };
                let chat_options = self.chat_render_options();
                let chat_is_focused = matches!(
                    self.active_screen,
                    Some(ActiveScreen::Chat { .. }) | Some(ActiveScreen::InBedChat(_))
                );
                let chat_hover = self.chat_component.render(
                    &mut self.renderer,
                    &self.font,
                    logical_h,
                    self.gui_ticks,
                    chat_mode,
                    chat_options,
                    self.mouse_x.floor(),
                    self.mouse_y.floor(),
                    self.shift_held,
                );
                if chat_is_focused {
                    self.pending_chat_tooltip = chat_hover
                        .tag_tooltip
                        .or(chat_hover.queue_tooltip)
                        .or(chat_hover.restricted_tooltip);
                    self.pending_chat_hover = chat_hover.hover_action;
                    if chat_hover.pointing_hand {
                        self.pending_cursor = el_gui::input::events::CursorType::PointingHand;
                    }
                }
            }
            if self.show_player_list {
                let mut players: Vec<&crate::ui::scoreboard::PlayerInfoState> = self
                    .player_infos
                    .values()
                    .filter(|info| info.listed)
                    .collect();
                crate::ui::scoreboard::sort_tab_list_refs(&self.scoreboard, &mut players);
                players.truncate(80);
                scoreboard_hud::render_tab_list(
                    &mut self.renderer,
                    &self.font,
                    &self.scoreboard,
                    &players,
                    &self.tab_list_header,
                    &self.tab_list_footer,
                    logical_w,
                    hud_bg,
                    &mut self.tab_health_states,
                    self.gui_ticks,
                    self.online_mode,
                    &self.tablist_level_players,
                );
            }
        }
        profiler.observe("GuiManager::prepare::hud", hud_started_at);
        let screen_started_at = profiler.started();
        if let Some(mut screen) = self
            .active_screen
            .take()
            .filter(|screen| !matches!(screen, ActiveScreen::NavigationTransition))
        {
            if let Some(migrated) = screen.migrated_screen_mut() {
                if !migrated.base_ref().initialized() {
                    migrated
                        .base()
                        .set_last_input_keyboard(self.last_gui_input_type.is_keyboard());
                }
            }
            let screen_tag = match &screen {
                ActiveScreen::Title(_) => 1,
                ActiveScreen::NavigationTransition => 0,
                ActiveScreen::Pause(_) => 3,
                ActiveScreen::Options(_) => 4,
                ActiveScreen::Video(_) => 5,
                ActiveScreen::SkinCustomization(_) => 6,
                ActiveScreen::ChatOptions(_) => 7,
                ActiveScreen::SoundOptions(_) => 8,
                ActiveScreen::WorldOptions(_) => 9,
                ActiveScreen::Controls(_) => 10,
                ActiveScreen::CreditsAndAttribution(_) => 11,
                ActiveScreen::WinScreen { .. } => 37,
                ActiveScreen::DeathScreen(_) => 38,
                ActiveScreen::Stats(_) => 39,
                ActiveScreen::Advancements(_) => 41,
                ActiveScreen::OnlineOptions(_) => 40,
                ActiveScreen::TelemetryInfo(_) => 42,
                ActiveScreen::ServerLinks(_) => 43,
                ActiveScreen::ServerDialog { .. } => 45,
                ActiveScreen::GameRules(_) => 44,
                ActiveScreen::AccessibilityOptions(_) => 12,
                ActiveScreen::MouseSettings(_) => 13,
                ActiveScreen::Restrictions(_) => 14,
                ActiveScreen::LanguageSelect(_) => 15,
                ActiveScreen::FontSettings(_) => 16,
                ActiveScreen::KeyBinds { .. } => 17,
                ActiveScreen::SubmenuStub { .. } => 18,
                ActiveScreen::ConfirmLink(_) => 35,
                ActiveScreen::PackConfirm(_) => 36,
                ActiveScreen::Chat { .. } => 19,
                ActiveScreen::InBedChat(_) => 20,
                ActiveScreen::Container(_) => 21,
                ActiveScreen::Multiplayer(_) => 22,
                ActiveScreen::ManageServer(_) => 23,
                ActiveScreen::DirectConnect(_) => 24,
                ActiveScreen::Connecting(_) => 25,
                ActiveScreen::LevelLoading(_) => 26,
                ActiveScreen::Disconnected(_) => 27,
                ActiveScreen::Safety(_) => 28,
                ActiveScreen::CodeOfConduct(_) => 29,
                ActiveScreen::Confirm(_) => 30,
                ActiveScreen::FriendsConfirm { .. } => 46,
                ActiveScreen::ServerReconfig(_) => 31,
                ActiveScreen::ResourcePacks(_) => 32,
                ActiveScreen::ExtensionCatalog { .. } => 33,
                ActiveScreen::ExtensionSettings { .. } => 34,
            };
            let mut anim_phase = 0u64;
            let mut extra_hash = 0u64;
            match &screen {
                ActiveScreen::Chat { input } => {
                    anim_phase = ((self.gui_ticks / 6) % 2) as u64;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    input.hash(&mut hasher);
                    self.chat_cursor.hash(&mut hasher);
                    self.chat_selection_anchor.hash(&mut hasher);
                    self.chat_suggestions_popup_open.hash(&mut hasher);
                    self.chat_suggestion_index.hash(&mut hasher);
                    self.chat_suggestion_offset.hash(&mut hasher);
                    self.chat_is_draft.hash(&mut hasher);
                    self.chat_input_display_pos.hash(&mut hasher);
                    self.shift_held.hash(&mut hasher);
                    extra_hash = hasher.finish();
                }
                ActiveScreen::InBedChat(in_bed) => {
                    anim_phase = ((self.gui_ticks / 6) % 2) as u64;
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    in_bed.input.hash(&mut hasher);
                    self.chat_cursor.hash(&mut hasher);
                    self.chat_selection_anchor.hash(&mut hasher);
                    self.chat_suggestions_popup_open.hash(&mut hasher);
                    self.chat_suggestion_index.hash(&mut hasher);
                    self.chat_suggestion_offset.hash(&mut hasher);
                    self.chat_is_draft.hash(&mut hasher);
                    self.chat_input_display_pos.hash(&mut hasher);
                    self.shift_held.hash(&mut hasher);
                    extra_hash = hasher.finish();
                }
                ActiveScreen::Title(_) => {
                    if !self.rendering_in_world && !self.panorama_already_rendered {
                        anim_phase = self.panorama_spin.to_bits() as u64;
                    }
                }
                ActiveScreen::ExtensionCatalog { screen, .. } => {
                    extra_hash = screen.page() as u64;
                }
                ActiveScreen::ExtensionSettings { owner, parent } => {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    owner.hash(&mut hasher);
                    extension_menus::settings_title().hash(&mut hasher);
                    extension_menus::settings_source_footer().hash(&mut hasher);
                    parent
                        .as_ref()
                        .is_some_and(|screen| screen.is_in_world())
                        .hash(&mut hasher);
                    extra_hash = hasher.finish();
                }
                ActiveScreen::ServerDialog { screen, .. } => {
                    extra_hash = screen.page() as u64 | ((screen.waiting_phase() as u64) << 32);
                }
                ActiveScreen::LevelLoading(_) => {
                    anim_phase = self.gui_ticks as u64;
                }
                ActiveScreen::LanguageSelect(_)
                | ActiveScreen::Confirm(_)
                | ActiveScreen::ServerReconfig(_)
                | ActiveScreen::ResourcePacks(_) => {
                    anim_phase = self.gui_ticks as u64;
                }
                ActiveScreen::DeathScreen(screen) => {
                    anim_phase = screen.delay_ticker() as u64;
                }
                ActiveScreen::WinScreen { screen, .. } => {
                    anim_phase = win_screen_cache_phase(screen);
                }
                ActiveScreen::Container(container_screen) => {
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    [self.shift_held, self.control_held, self.alt_held].hash(&mut hasher);
                    container_screen.hovered_slot_idx.hash(&mut hasher);
                    container_screen.quick_craft_slots.len().hash(&mut hasher);
                    container_screen.menu.slots.len().hash(&mut hasher);
                    container_screen
                        .carried_item
                        .item
                        .location
                        .to_string()
                        .hash(&mut hasher);
                    container_screen.carried_item.count.hash(&mut hasher);
                    player.inventory.len().hash(&mut hasher);
                    player.age_in_ticks.to_bits().hash(&mut hasher);
                    extra_hash = hasher.finish();
                }
                _ => {}
            }
            let state_key = ScreenCacheKey {
                screen_tag,
                logical_w: logical_w.to_bits(),
                logical_h: logical_h.to_bits(),
                mouse_x: self.mouse_x.floor() as i32,
                mouse_y: self.mouse_y.floor() as i32,
                anim_phase,
                fov_bits: self.fov_setting.to_bits(),
                hide_server_address: self.option_bool("hideServerAddress", false),
                defer_inventory_tooltip,
                custom_textures_len: self.renderer.custom_textures.len(),
                extra_hash,
            };
            let can_reuse_cache =
                !self.screen_dirty && self.screen_cache.state_key.as_ref() == Some(&state_key);
            #[cfg(feature = "extensions")]
            let can_reuse_cache = can_reuse_cache
                && !(matches!(screen, ActiveScreen::Container(_))
                    && crate::modding::hooks::item_tooltip_active());
            let live_chat_cursor = matches!(
                screen,
                ActiveScreen::Chat { .. } | ActiveScreen::InBedChat(_)
            )
            .then_some(self.pending_cursor);
            if live_chat_cursor.is_some() {
                self.pending_cursor = el_gui::input::events::CursorType::Default;
            }
            if can_reuse_cache {
                self.renderer.seal_batches();
                self.renderer.append_geometry(
                    &self.screen_cache.vertices,
                    &self.screen_cache.indices,
                    &self.screen_cache.batches,
                );
                self.pending_cursor = self.screen_cache.pending_cursor;
                self.renderer.seal_batches();
                self.active_screen = Some(screen);
            } else {
                self.renderer.seal_batches();
                let v_start = self.renderer.vertices.len();
                let i_start = self.renderer.indices.len();
                let b_start = self.renderer.batches.len();
                match &mut screen {
                    ActiveScreen::Title(title_screen) => {
                        use el_gui::framework::screen::Screen;
                        title_screen.set_extensions(
                            self.extension_catalog.mods_enabled,
                            self.extension_catalog.shaderpacks_enabled,
                        );
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            title_screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        title_screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Pause(pause_screen) => {
                        use el_gui::framework::screen::Screen;
                        pause_screen.set_extensions(
                            self.extension_catalog.mods_enabled,
                            self.extension_catalog.shaderpacks_enabled,
                        );
                        let (available, title) = self.pause_screen_addition();
                        pause_screen.set_custom_addition(available, title);
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            pause_screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        pause_screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::FriendsConfirm { screen, .. } => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Options(options_screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            options_screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        options_screen.set_fov_value(self.fov_setting);
                        options_screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Video(video_screen) => {
                        use el_gui::framework::screen::Screen;
                        video_screen
                            .set_shaderpacks_enabled(self.extension_catalog.shaderpacks_enabled);
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            video_screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        video_screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::SkinCustomization(screen) => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::ChatOptions(screen) => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::SoundOptions(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(&mut graphics, &self.font);
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::OnlineOptions(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::TelemetryInfo(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            partial_tick,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::ServerLinks(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            partial_tick,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::ServerDialog { screen, .. } => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            partial_tick,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::WorldOptions(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        screen.extract_render_state(&mut graphics, &self.font);
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::GameRules(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            partial_tick,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Controls(screen) => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::CreditsAndAttribution(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::WinScreen { screen, .. } => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            partial_tick,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::DeathScreen(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            partial_tick,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Stats(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            partial_tick,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Advancements(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            partial_tick,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                        self.collect_seen_advancement_actions(screen);
                    }
                    ActiveScreen::Multiplayer(screen) => {
                        let trace_first_render = !screen.base_ref().initialized();
                        if trace_first_render {
                            ui_crash_trace(format!(
                                "Multiplayer stage=first_render_begin in_world={} size={}x{}",
                                screen.in_world,
                                logical_w.round() as i32,
                                logical_h.round() as i32,
                            ));
                        }
                        screen.hide_server_address = self.option_bool("hideServerAddress", false);
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        if trace_first_render {
                            ui_crash_trace("Multiplayer stage=screen_init_done");
                        }
                        Screen::extract_render_state(
                            screen,
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        if trace_first_render {
                            ui_crash_trace("Multiplayer stage=extract_render_state_done");
                        }
                        let pending_pings = screen.take_pending_ping_entries();
                        #[cfg(not(target_arch = "wasm32"))]
                        for entry in pending_pings {
                            self.ping_single_server(entry);
                        }
                        #[cfg(target_arch = "wasm32")]
                        if !pending_pings.is_empty() {
                            self.start_pinging_servers(&pending_pings);
                        }
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        if trace_first_render {
                            ui_crash_trace("Multiplayer stage=renderer_begin");
                        }
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                        if trace_first_render {
                            ui_crash_trace("Multiplayer stage=renderer_done");
                        }
                    }
                    ActiveScreen::ManageServer(screen) => {
                        let trace_first_render = !screen.base_ref().initialized();
                        if trace_first_render {
                            ui_crash_trace(format!(
                                "AddServer stage=first_render_begin in_world={} size={}x{}",
                                screen.in_world(),
                                logical_w.round() as i32,
                                logical_h.round() as i32,
                            ));
                        }
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        if trace_first_render {
                            ui_crash_trace("AddServer stage=screen_init_done");
                        }
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        if trace_first_render {
                            ui_crash_trace("AddServer stage=extract_render_state_done");
                        }
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        if trace_first_render {
                            ui_crash_trace("AddServer stage=render_state_finished");
                            ui_crash_trace("AddServer stage=renderer_begin");
                        }
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                        if trace_first_render {
                            ui_crash_trace("AddServer stage=renderer_done");
                        }
                    }
                    ActiveScreen::DirectConnect(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Disconnected(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::ConfirmLink(screen) => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::PackConfirm(screen) => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Confirm(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::ServerReconfig(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Safety(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::CodeOfConduct(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Connecting(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::LevelLoading(_) => {
                        self.render_level_loading_screen(logical_w, logical_h);
                    }
                    ActiveScreen::AccessibilityOptions(screen) => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::MouseSettings(screen) => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::Restrictions(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        screen.extract_render_state(&mut graphics, &self.font);
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::LanguageSelect(screen) => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::FontSettings(screen) => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        screen.extract_render_state(&mut graphics, &self.font);
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::KeyBinds { screen, .. } => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::ResourcePacks(screen) => {
                        use el_gui::framework::screen::Screen;
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        prepare_screen_for_render(
                            screen,
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                        );
                        screen.extract_render_state(
                            &mut graphics,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                            0.0,
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::ExtensionCatalog { screen, .. } => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        screen.extract_render_state(&mut graphics, &self.font);
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::ExtensionSettings { parent, .. } => {
                        let mut graphics = GuiGraphicsExtractor::new(
                            logical_w.round() as i32,
                            logical_h.round() as i32,
                            self.mouse_x.floor() as i32,
                            self.mouse_y.floor() as i32,
                        );
                        extension_menus::render_settings_shell(
                            &mut graphics,
                            &self.font,
                            parent.as_ref().is_some_and(|screen| screen.is_in_world()),
                        );
                        self.pending_cursor = graphics.pending_cursor();
                        let state = graphics.finish();
                        self.render_eltharn_gui_state(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &state,
                        );
                    }
                    ActiveScreen::SubmenuStub { submenu, in_world } => {
                        self.render_submenu_stub(
                            device,
                            queue,
                            texture_bind_group_layout,
                            logical_w,
                            logical_h,
                            *submenu,
                            *in_world,
                        );
                    }
                    ActiveScreen::Chat { input } => {
                        let input = input.clone();
                        self.render_chat_input_and_suggestions(
                            device,
                            queue,
                            texture_bind_group_layout,
                            logical_w,
                            logical_h,
                            &input,
                            None,
                        );
                    }
                    ActiveScreen::InBedChat(screen) => {
                        let input = screen.input.clone();
                        self.render_chat_input_and_suggestions(
                            device,
                            queue,
                            texture_bind_group_layout,
                            logical_w,
                            logical_h,
                            &input,
                            Some(screen),
                        );
                    }
                    ActiveScreen::NavigationTransition => {}
                    ActiveScreen::Container(container_screen) => {
                        #[cfg(feature = "extensions")]
                        {
                            container_screen.tooltip_modifiers =
                                [self.shift_held, self.control_held, self.alt_held];
                        }
                        container_screen.render(
                            &mut self.renderer,
                            &self.font,
                            device,
                            queue,
                            texture_bind_group_layout,
                            &self.resource_manager,
                            logical_w,
                            logical_h,
                            self.mouse_x,
                            self.mouse_y,
                            defer_inventory_tooltip,
                            player,
                        );
                        self.pending_bundle_item_selections
                            .extend(container_screen.take_pending_bundle_item_selections());
                    }
                }
                let new_vertices = self.renderer.vertices[v_start..].to_vec();
                let new_indices: Vec<u32> = self.renderer.indices[i_start..]
                    .iter()
                    .map(|&idx| idx.saturating_sub(v_start as u32))
                    .collect();
                let mut new_batches = Vec::new();
                for batch in &self.renderer.batches[b_start..] {
                    let mut normalized = batch.clone();
                    match &mut normalized {
                        DrawBatch::Solid { start_idx, .. }
                        | DrawBatch::Color { start_idx, .. }
                        | DrawBatch::ColorInvertRect { start_idx, .. }
                        | DrawBatch::ColorAdditiveRect { start_idx, .. }
                        | DrawBatch::GuiAtlas { start_idx, .. }
                        | DrawBatch::GuiGlintAtlas { start_idx, .. }
                        | DrawBatch::Crosshair { start_idx, .. }
                        | DrawBatch::FontSheet { start_idx, .. }
                        | DrawBatch::CustomTexture { start_idx, .. }
                        | DrawBatch::ItemCutout { start_idx, .. }
                        | DrawBatch::GuiGlintCustomTexture { start_idx, .. }
                        | DrawBatch::EndPortal { start_idx, .. }
                        | DrawBatch::Vignette { start_idx, .. } => {
                            *start_idx = start_idx.saturating_sub(i_start as u32);
                        }
                    }
                    new_batches.push(normalized);
                }
                self.screen_cache.state_key = Some(state_key);
                self.screen_cache.vertices = new_vertices;
                self.screen_cache.indices = new_indices;
                self.screen_cache.batches = new_batches;
                self.screen_cache.pending_cursor = self.pending_cursor;
                self.screen_cache.pending_chat_tooltip = None;
                self.screen_dirty = false;
                self.renderer.seal_batches();
                self.active_screen = Some(screen);
            }
            self.pending_cursor =
                screen_cursor_over_live_chat(live_chat_cursor, self.pending_cursor);
        } else {
            self.screen_cache.state_key = None;
        }
        profiler.observe("GuiManager::prepare::screen", screen_started_at);
        #[cfg(feature = "extensions")]
        self.render_mod_app(
            device,
            queue,
            texture_bind_group_layout,
            logical_w as i32,
            logical_h as i32,
        );
        #[cfg(feature = "extensions")]
        {
            let focus = crate::modding::input::focus();
            if let Err(error) = self.render_mod_content(
                device,
                queue,
                texture_bind_group_layout,
                logical_w as i32,
                logical_h as i32,
                focus.as_ref().map(|(owner, id)| (owner.as_str(), *id)),
            ) {
                crate::modding::fail(error);
            }
        }
        self.render_now_playing_toast(
            device,
            queue,
            texture_bind_group_layout,
            logical_w,
            logical_h,
        );
        let deferred_started_at = profiler.started();
        if let Some(tooltip) = self.pending_chat_tooltip.take() {
            self.draw_chat_tooltip(&tooltip, logical_w, logical_h);
        } else if let Some(hover) = self.pending_chat_hover.take() {
            self.draw_chat_hover(&hover, logical_w, logical_h, player);
        }
        profiler.observe("GuiManager::prepare::screen_deferred", deferred_started_at);
        let upload_started_at = profiler.started();
        self.renderer.end_frame(device, queue);
        profiler.observe("GuiManager::prepare::upload", upload_started_at);
        profiler.observe("GuiManager::prepare", prepare_started_at);
    }
    #[cfg(target_os = "android")]
    fn render_android_controls(&mut self, width: f32, height: f32) {
        let scale = self
            .get_gui_scale_factor(self.last_screen_w, self.last_screen_h)
            .max(1) as f32;
        let insets = self.touch_safe_insets.map(|value| value / scale);
        let Ok(layout) = self
            .touch_controls
            .resolve_safe_viewport(width, height, insets)
        else {
            return;
        };
        if !layout.enabled {
            return;
        }
        let opacity = layout.opacity;
        let pad = layout.joystick;
        self.renderer.draw_circle(
            pad.center[0],
            pad.center[1],
            pad.radius,
            [0.035, 0.055, 0.085, opacity * 0.68],
        );
        self.renderer.draw_circle_outline(
            pad.center[0],
            pad.center[1],
            pad.radius,
            pad.radius * 0.975,
            [0.72, 0.84, 0.95, opacity * 0.56],
        );
        self.renderer.draw_circle_outline(
            pad.center[0],
            pad.center[1],
            pad.radius * 0.72,
            pad.radius * 0.705,
            [0.52, 0.68, 0.82, opacity * 0.22],
        );
        let offset = self.touch_visual.joystick_offset;
        let knob_x = pad.center[0] + offset[0] * pad.radius * 0.58;
        let knob_y = pad.center[1] + offset[1] * pad.radius * 0.58;
        let knob = pad.radius * 0.36;
        let active = self.touch_visual.joystick_active;
        self.renderer.draw_circle(
            knob_x,
            knob_y + knob * 0.12,
            knob * 1.08,
            [0.0, 0.0, 0.0, opacity * 0.22],
        );
        self.renderer.draw_circle(
            knob_x,
            knob_y,
            knob,
            if active {
                [0.23, 0.78, 0.79, opacity]
            } else {
                [0.26, 0.37, 0.47, opacity * 0.95]
            },
        );
        self.renderer.draw_circle_outline(
            knob_x,
            knob_y,
            knob,
            knob * 0.95,
            [0.78, 0.96, 1.0, opacity * 0.8],
        );
        self.renderer.draw_circle(
            knob_x - knob * 0.19,
            knob_y - knob * 0.22,
            knob * 0.12,
            [0.9, 1.0, 1.0, opacity * 0.36],
        );
        for button in &layout.buttons {
            let pressed = self.touch_visual.pressed.contains(&button.action);
            let [x, y] = button.circle.center;
            let radius = button.circle.radius;
            self.renderer.draw_circle(
                x,
                y + radius * 0.08,
                radius * 1.04,
                [0.0, 0.0, 0.0, opacity * 0.18],
            );
            self.renderer.draw_circle(
                x,
                y,
                radius,
                if pressed {
                    [0.17, 0.65, 0.69, opacity]
                } else {
                    [0.045, 0.075, 0.11, opacity * 0.78]
                },
            );
            self.renderer.draw_circle_outline(
                x,
                y,
                radius,
                radius * 0.96,
                if pressed {
                    [0.72, 1.0, 1.0, opacity]
                } else {
                    [0.66, 0.78, 0.9, opacity * 0.65]
                },
            );
            let icon_scale = radius * 0.62;
            let thickness = (radius * 0.09).max(0.7);
            for [x0, y0, x1, y1] in &button.glyph {
                let a = [x + x0 * icon_scale, y + y0 * icon_scale];
                let b = [x + x1 * icon_scale, y + y1 * icon_scale];
                let dx = b[0] - a[0];
                let dy = b[1] - a[1];
                let length = dx.hypot(dy).max(0.001);
                let nx = -dy / length * thickness * 0.5;
                let ny = dx / length * thickness * 0.5;
                self.renderer.draw_color_mesh_quad(
                    [
                        [a[0] + nx, a[1] + ny],
                        [a[0] - nx, a[1] - ny],
                        [b[0] - nx, b[1] - ny],
                        [b[0] + nx, b[1] + ny],
                    ],
                    [[0.9, 0.97, 1.0, opacity]; 4],
                );
                self.renderer
                    .draw_circle(a[0], a[1], thickness * 0.5, [0.9, 0.97, 1.0, opacity]);
                self.renderer
                    .draw_circle(b[0], b[1], thickness * 0.5, [0.9, 0.97, 1.0, opacity]);
            }
        }
    }
    pub fn render_prepared<'rp>(&'rp self, render_pass: &mut RenderPass<'rp>, skip_color: bool) {
        if skip_color {
            self.renderer.render_without_color(render_pass, &self.font);
        } else {
            self.renderer.render(render_pass, &self.font);
        }
    }
    pub fn render_color_batches<'rp>(&'rp self, render_pass: &mut RenderPass<'rp>) {
        self.renderer.render_color_batches(render_pass);
    }
    pub fn render_deferred_inventory_tooltip<'rp>(
        &'rp mut self,
        device: &Device,
        queue: &Queue,
        render_pass: &mut RenderPass<'rp>,
    ) {
        let Some(ActiveScreen::Container(screen)) = self.active_screen.as_ref() else {
            return;
        };
        let Some(tooltip) = screen.deferred_tooltip() else {
            return;
        };
        let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
        let logical_w = self.last_screen_w / scale;
        let logical_h = self.last_screen_h / scale;
        let (glint_speed, glint_strength) = self.enchantment_glint_options();
        self.renderer.start_frame_with_glint(
            queue,
            logical_w.max(1.0),
            logical_h.max(1.0),
            glint_speed,
            glint_strength,
        );
        crate::ui::gui::screen::container::render_tooltip(
            &mut self.renderer,
            &self.font,
            tooltip,
            self.mouse_x,
            self.mouse_y,
            logical_w,
            logical_h,
        );
        self.renderer.end_frame(device, queue);
        self.renderer.render(render_pass, &self.font);
    }
    fn render_level_loading_screen(&mut self, logical_w: f32, logical_h: f32) {
        self.renderer.draw_rect(
            0.0,
            0.0,
            logical_w,
            logical_h,
            Self::level_loading_menu_tile_color(),
        );
        let title = crate::ui::i18n::translate("multiplayer.downloadingTerrain");
        let title_w = self.font.measure_width(&title, 1.0);
        self.renderer.draw_text_top_with_shadow(
            &self.font,
            &title,
            (logical_w - title_w) * 0.5,
            logical_h * 0.5 - 50.0,
            [1.0, 1.0, 1.0, 1.0],
            1.0,
        );
    }
    fn level_loading_menu_tile_color() -> [f32; 4] {
        [0.0, 0.0, 0.0, MENU_BACKGROUND_ALPHA]
    }
    fn render_submenu_stub(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        logical_w: f32,
        logical_h: f32,
        submenu: PauseSubmenu,
        in_world: bool,
    ) {
        let mut graphics = GuiGraphicsExtractor::new(
            logical_w.round() as i32,
            logical_h.round() as i32,
            self.mouse_x.floor() as i32,
            self.mouse_y.floor() as i32,
        );
        if in_world {
            graphics.blur_before_this_stratum();
            graphics.next_stratum();
        }
        let background = ResourceLocation::from_string(if in_world {
            "minecraft:gui/inworld_menu_background"
        } else {
            "minecraft:gui/menu_background"
        });
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        let mut y = 0;
        while y < height {
            let tile_h = 32.min(height - y);
            let mut x = 0;
            while x < width {
                let tile_w = 32.min(width - x);
                graphics.blit_region(
                    background.clone(),
                    x,
                    y,
                    tile_w,
                    tile_h,
                    0.0,
                    0.0,
                    tile_w as f32 / 32.0,
                    tile_h as f32 / 32.0,
                    0xFFFF_FFFFu32 as i32,
                );
                x += 32;
            }
            y += 32;
        }
        let button_w = 200;
        let button_h = 20;
        let button_x = (width - button_w) / 2;
        let button_y = height - 28;
        graphics.draw_button_with_focus(
            &self.font,
            button_x,
            button_y,
            button_w,
            button_h,
            &crate::ui::i18n::translate("gui.done"),
            true,
            true,
        );
        self.pending_cursor = graphics.pending_cursor();
        let state = graphics.finish();
        self.render_eltharn_gui_state(device, queue, texture_bind_group_layout, &state);
        let title = crate::ui::i18n::translate(submenu.title_key());
        let title_w = self.font.measure_width(&title, 1.0);
        self.renderer.draw_text_top_with_shadow(
            &self.font,
            &title,
            (logical_w - title_w) * 0.5,
            48.0,
            [1.0, 1.0, 1.0, 1.0],
            1.0,
        );
        let message_key = match submenu {
            PauseSubmenu::Singleplayer => "eltharn.menu.singleplayer.unavailable",
            PauseSubmenu::Friends => "eltharn.menu.friends.unavailable",
            PauseSubmenu::Realms => "eltharn.menu.realms.unavailable",
            PauseSubmenu::Telemetry => "eltharn.menu.telemetry.unavailable",
            PauseSubmenu::GameRulesUnavailable => "eltharn.menu.game_rules.unavailable",
            PauseSubmenu::Credits => "eltharn.menu.credits.unavailable",
            PauseSubmenu::DemoReset => "eltharn.menu.demo_reset.unavailable",
            _ => "eltharn.menu.unavailable",
        };
        let message = crate::ui::i18n::translate(message_key);
        let lines = el_gui::wrap_text(&message, (logical_w - 40.0).max(80.0), &self.font);
        let line_height = 10.0;
        let first_line_y = logical_h * 0.5 - lines.len() as f32 * line_height * 0.5;
        for (index, line) in lines.iter().enumerate() {
            let line_w = self.font.measure_width(line, 1.0);
            self.renderer.draw_text_top_with_shadow(
                &self.font,
                line,
                (logical_w - line_w) * 0.5,
                first_line_y + index as f32 * line_height,
                [0.75, 0.75, 0.75, 1.0],
                1.0,
            );
        }
    }
    fn draw_chat_hover(
        &mut self,
        hover: &crate::ui::domain::ChatHoverAction,
        logical_w: f32,
        logical_h: f32,
        player: &LocalPlayer,
    ) {
        use crate::ui::domain::ChatHoverAction;
        match hover {
            ChatHoverAction::ShowText { value } => {
                let lines = crate::ui::chat::chat_component::split_formatted_chat_lines(
                    &self.font,
                    value,
                    (logical_w / 2.0).max(200.0),
                );
                self.draw_chat_tooltip(&lines.join("\n"), logical_w, logical_h);
            }
            ChatHoverAction::ShowEntity { id, uuid, name } => {
                if !self.option_bool("advancedItemTooltips", false) {
                    return;
                }
                let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
                let description_key = format!("entity.{namespace}.{}", path.replace('/', "."));
                let description = crate::ui::i18n::translate(&description_key);
                let type_line = crate::ui::i18n::translate("gui.entity_tooltip.type")
                    .replace("%s", &description);
                let mut lines = Vec::new();
                if let Some(name) = name {
                    lines.push(name.clone());
                }
                lines.push(type_line);
                lines.push(uuid.clone());
                self.draw_chat_tooltip(&lines.join("\n"), logical_w, logical_h);
            }
            ChatHoverAction::ShowItem { id, count, tag } => {
                let Some(item) = chat_hover_item_stack(id, *count, tag.as_deref()) else {
                    return;
                };
                let attribute_base = |definition: crate::gameplay::attributes::RangedAttribute| {
                    player
                        .attributes
                        .get(definition.id)
                        .map(|attribute| attribute.base)
                        .unwrap_or(definition.default_value)
                };
                let tooltip = crate::ui::gui::screen::container::chat_item_tooltip(
                    &item,
                    self.option_bool("advancedItemTooltips", false),
                    Some((
                        attribute_base(crate::gameplay::attributes::ATTACK_DAMAGE),
                        attribute_base(crate::gameplay::attributes::ATTACK_SPEED),
                    )),
                );
                if let Some(tooltip) = tooltip {
                    crate::ui::gui::screen::container::render_tooltip(
                        &mut self.renderer,
                        &self.font,
                        &tooltip,
                        self.mouse_x,
                        self.mouse_y,
                        logical_w,
                        logical_h,
                    );
                }
            }
        }
    }
    fn draw_chat_tooltip(&mut self, tooltip: &str, logical_w: f32, logical_h: f32) {
        let [text_width, text_height] = self.text_tooltip_size(tooltip);
        let mut x = self.mouse_x + 12.0;
        if x + text_width > logical_w {
            x = (x - 24.0 - text_width).max(4.0);
        }
        let mut y = self.mouse_y - 12.0;
        if y + text_height + 3.0 > logical_h {
            y = logical_h - text_height - 3.0;
        }
        self.draw_text_tooltip_at(tooltip, x, y);
    }
    fn text_tooltip_size(&self, tooltip: &str) -> [f32; 2] {
        let mut lines = tooltip.lines().collect::<Vec<_>>();
        if lines.is_empty() {
            lines.push(tooltip.trim());
        }
        let text_width = lines
            .iter()
            .map(|line| {
                crate::ui::chat::chat_component::formatted_chat_width(&self.font, line, 1.0)
            })
            .fold(0.0_f32, f32::max)
            .ceil();
        let text_height = if lines.len() == 1 {
            8.0
        } else {
            lines.len() as f32 * 10.0
        };
        [text_width, text_height]
    }
    fn draw_text_tooltip_at(&mut self, tooltip: &str, x: f32, y: f32) {
        let [text_width, text_height] = self.text_tooltip_size(tooltip);
        let mut lines = tooltip.lines().collect::<Vec<_>>();
        if lines.is_empty() {
            lines.push(tooltip.trim());
        }
        let background = ResourceLocation::new("minecraft", "tooltip/background");
        let frame = ResourceLocation::new("minecraft", "tooltip/frame");
        self.renderer.draw_sprite(
            x - 12.0,
            y - 12.0,
            text_width + 24.0,
            text_height + 24.0,
            &background,
            [1.0, 1.0, 1.0, 1.0],
        );
        self.renderer.draw_sprite(
            x - 12.0,
            y - 12.0,
            text_width + 24.0,
            text_height + 24.0,
            &frame,
            [1.0, 1.0, 1.0, 1.0],
        );
        for (index, line) in lines.iter().enumerate() {
            crate::ui::chat::chat_component::draw_formatted_chat_text_ex(
                &mut self.renderer,
                &self.font,
                line,
                x,
                y + if index == 0 {
                    0.0
                } else if index == 1 {
                    12.0
                } else {
                    12.0 + (index - 1) as f32 * 10.0
                },
                1.0,
                1.0,
                true,
            );
        }
    }
    fn render_now_playing_toast(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        logical_w: f32,
        logical_h: f32,
    ) {
        let Some(title) = self.now_playing_title.clone() else {
            return;
        };
        let display_mode = match self.extra_options.get("musicToast").map(String::as_str) {
            Some("pause") => 1,
            Some("pause_and_toast") => 2,
            _ => 0,
        };
        if display_mode == 0 {
            return;
        }
        let is_pause_screen = matches!(self.active_screen, Some(ActiveScreen::Pause(_)));
        let sound_volumes =
            crate::client::sound_engine::SoundSourceVolumes::from_options(&self.extra_options);
        let pause_inline = is_pause_screen
            && display_mode >= 1
            && sound_volumes.final_source_volume(crate::client::sound_engine::SoundSource::Music)
                > 0.0;
        let floating_started_at = self.now_playing_started_at;
        let Some(started_at) = floating_started_at else {
            if !pause_inline {
                return;
            }
            return;
        };
        let elapsed = started_at.elapsed();
        let display_ms = (5_000.0
            * self
                .option_float("notificationDisplayTime", 1.0)
                .clamp(0.5, 10.0)) as u128;
        let floating = !is_pause_screen && display_mode == 2 && elapsed.as_millis() < display_ms;
        if !pause_inline && !floating {
            return;
        }
        let screen_width = logical_w.round() as i32;
        let screen_height = logical_h.round() as i32;
        if screen_width < 40 || screen_height < 30 {
            return;
        }
        let max_text_width = (screen_width - 44).max(0) as f32;
        let mut visible_title = title;
        while self.font.measure_width(&visible_title, 1.0) > max_text_width
            && !visible_title.is_empty()
        {
            visible_title.pop();
        }
        if visible_title != self.now_playing_title.as_deref().unwrap_or_default()
            && !visible_title.is_empty()
        {
            while self.font.measure_width(&format!("{visible_title}…"), 1.0) > max_text_width
                && !visible_title.is_empty()
            {
                visible_title.pop();
            }
            visible_title.push('…');
        }
        let width = (37.0 + self.font.measure_width(&visible_title, 1.0))
            .ceil()
            .clamp(40.0, screen_width as f32) as i32;
        let x = if pause_inline {
            0
        } else {
            let enter_progress = (elapsed.as_millis() as f32 / 250.0).clamp(0.0, 1.0);
            (screen_width as f32 - width as f32 * enter_progress).round() as i32
        };
        let mut graphics = GuiGraphicsExtractor::new(
            screen_width,
            screen_height,
            self.mouse_x.floor() as i32,
            self.mouse_y.floor() as i32,
        );
        graphics.blit_resource_region(
            ResourceLocation::from_string("minecraft:gui/sprites/toast/now_playing"),
            x,
            0,
            width,
            30,
            0.0,
            0.0,
            1.0,
            1.0,
            -1,
        );
        graphics.blit_resource_region(
            ResourceLocation::from_string("minecraft:gui/sprites/icon/music_notes"),
            x + 7,
            7,
            16,
            16,
            0.0,
            0.0,
            1.0,
            1.0,
            0xFFB8DDF2u32 as i32,
        );
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            visible_title,
            x + 30,
            10,
            0xFFB0B0B0u32 as i32,
        );
        let render_state = graphics.finish();
        self.render_eltharn_gui_state(device, queue, texture_bind_group_layout, &render_state);
    }
    fn render_eltharn_gui_state(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        state: &GuiRenderState,
    ) {
        if !self.rendering_in_world {
            if !self.panorama_already_rendered {
                let realtime_delta_ticks = self.advance_realtime_delta_ticks();
                let spin = self.advance_menu_panorama(realtime_delta_ticks);
                self.render_title_panorama(device, queue, texture_bind_group_layout, spin);
                if !self.rendering_title_screen {
                    self.render_panorama_overlay(device, queue, texture_bind_group_layout);
                }
            }
        } else if state.panorama_render_state.is_some() {
            self.render_title_panorama(
                device,
                queue,
                texture_bind_group_layout,
                self.panorama_spin,
            );
            self.render_panorama_overlay(device, queue, texture_bind_group_layout);
        }
        if state.has_blur_request() {
            self.render_eltharn_gui_range(
                device,
                queue,
                texture_bind_group_layout,
                state,
                TraverseRange::BeforeBlur,
            );
            self.render_eltharn_gui_range(
                device,
                queue,
                texture_bind_group_layout,
                state,
                TraverseRange::AfterBlur,
            );
        } else {
            self.render_eltharn_gui_range(
                device,
                queue,
                texture_bind_group_layout,
                state,
                TraverseRange::All,
            );
        }
        self.renderer.active_scissor = None;
    }
    fn render_title_panorama(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        spin: f32,
    ) {
        let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
        let logical_w = self.last_screen_w / scale;
        let logical_h = self.last_screen_h / scale;
        if logical_w <= 0.0 || logical_h <= 0.0 {
            return;
        }
        fn project_point(
            x: f32,
            y: f32,
            z: f32,
            yaw: f32,
            pitch: f32,
            logical_w: f32,
            logical_h: f32,
        ) -> Option<[f32; 2]> {
            let yaw_rad = yaw.to_radians();
            let pitch_rad = pitch.to_radians();
            let cos_y = yaw_rad.cos();
            let sin_y = yaw_rad.sin();
            let x1 = x * cos_y - z * sin_y;
            let z1 = x * sin_y + z * cos_y;
            let y1 = y;
            let cos_x = pitch_rad.cos();
            let sin_x = pitch_rad.sin();
            let x2 = x1;
            let y2 = y1 * cos_x - z1 * sin_x;
            let z2 = y1 * sin_x + z1 * cos_x;
            let x_final = x2;
            let y_final = -y2;
            let z_final = -z2;
            if z_final <= 0.01 {
                return None;
            }
            let fov = 85.0f32.to_radians();
            let scale_proj = 1.0 / (fov / 2.0).tan();
            let screen_x = (x_final / z_final) * scale_proj;
            let screen_y = (y_final / z_final) * scale_proj;
            let aspect = logical_w / logical_h;
            let sx = (screen_x / aspect + 1.0) * 0.5 * logical_w;
            let sy = (1.0 - screen_y) * 0.5 * logical_h;
            Some([sx, sy])
        }
        self.renderer
            .draw_rect(0.0, 0.0, logical_w, logical_h, [0.0, 0.0, 0.0, 1.0]);
        for face in 0..6 {
            let texture = ResourceLocation::from_string(&format!(
                "minecraft:gui/title/background/panorama_{}",
                face
            ));
            let texture_idx = self.renderer.get_or_load_custom_texture(
                device,
                queue,
                texture_bind_group_layout,
                &self.resource_manager,
                &texture,
            );
            let grid_size = 16;
            let mut projected_grid = vec![vec![None; grid_size + 1]; grid_size + 1];
            for i in 0..=grid_size {
                let u = i as f32 / grid_size as f32;
                for j in 0..=grid_size {
                    let v = j as f32 / grid_size as f32;
                    let (px, py, pz) = match face {
                        0 => (-1.0 + 2.0 * u, -1.0 + 2.0 * v, 1.0),
                        1 => (1.0, -1.0 + 2.0 * v, 1.0 - 2.0 * u),
                        2 => (1.0 - 2.0 * u, -1.0 + 2.0 * v, -1.0),
                        3 => (-1.0, -1.0 + 2.0 * v, -1.0 + 2.0 * u),
                        4 => (-1.0 + 2.0 * u, -1.0, -1.0 + 2.0 * v),
                        5 => (-1.0 + 2.0 * u, 1.0, 1.0 - 2.0 * v),
                        _ => unreachable!(),
                    };
                    projected_grid[i][j] =
                        project_point(px, py, pz, spin, 10.0, logical_w, logical_h);
                }
            }
            for i in 0..grid_size {
                let u0 = i as f32 / grid_size as f32;
                let u1 = (i + 1) as f32 / grid_size as f32;
                for j in 0..grid_size {
                    let v0 = j as f32 / grid_size as f32;
                    let v1 = (j + 1) as f32 / grid_size as f32;
                    if let (Some(p0), Some(p1), Some(p2), Some(p3)) = (
                        projected_grid[i][j],
                        projected_grid[i + 1][j],
                        projected_grid[i + 1][j + 1],
                        projected_grid[i][j + 1],
                    ) {
                        let indices_before = self.renderer.indices.len() as u32;
                        self.renderer.add_custom_quad(
                            p0,
                            p1,
                            p2,
                            p3,
                            [u0, v0],
                            [u1, v0],
                            [u1, v1],
                            [u0, v1],
                            [1.0, 1.0, 1.0, 1.0],
                        );
                        let draw_count = self.renderer.indices.len() as u32 - indices_before;
                        if draw_count == 0 {
                            continue;
                        }
                        if let Some(DrawBatch::CustomTexture {
                            bind_group_idx,
                            start_idx: _,
                            count,
                        }) = self.renderer.batches.last_mut()
                        {
                            if *bind_group_idx == texture_idx {
                                *count += draw_count;
                                continue;
                            }
                        }
                        self.renderer.batches.push(DrawBatch::CustomTexture {
                            bind_group_idx: texture_idx,
                            start_idx: indices_before,
                            count: draw_count,
                        });
                    }
                }
            }
        }
    }
    fn render_panorama_overlay(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
    ) {
        let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
        let logical_w = self.last_screen_w / scale;
        let logical_h = self.last_screen_h / scale;
        if logical_w <= 0.0 || logical_h <= 0.0 {
            return;
        }
        let overlay =
            ResourceLocation::from_string("minecraft:gui/title/background/panorama_overlay");
        let texture_idx = self.renderer.get_or_load_custom_texture(
            device,
            queue,
            texture_bind_group_layout,
            &self.resource_manager,
            &overlay,
        );
        self.renderer.draw_custom_texture_region(
            0.0,
            0.0,
            logical_w,
            logical_h,
            0.0,
            0.0,
            1.0,
            1.0,
            texture_idx,
            [1.0, 1.0, 1.0, 1.0],
        );
    }
    fn render_eltharn_gui_range(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        state: &GuiRenderState,
        range: TraverseRange,
    ) {
        state.visit_render_nodes(range, |elements, glyphs, texts, items| {
            for op in crate::ui::gui::item_renderer::merged_item_element_ops(elements, items) {
                match op {
                    crate::ui::gui::item_renderer::GuiFlowItemOp::Element(element) => {
                        self.render_eltharn_gui_element(
                            device,
                            queue,
                            texture_bind_group_layout,
                            element,
                        );
                    }
                    crate::ui::gui::item_renderer::GuiFlowItemOp::Item(index) => {
                        let item = &items[index];
                        self.apply_eltharn_scissor(item.scissor);
                        let (x, y) = transform_point(item.pose, item.x as f32, item.y as f32);
                        crate::ui::gui::item_renderer::draw_gui_item(
                            &mut self.renderer,
                            &self.font,
                            &item.item_stack,
                            x,
                            y,
                            0.0,
                            item.decorations,
                        );
                    }
                }
            }
            for glyph in glyphs {
                self.render_eltharn_gui_element(device, queue, texture_bind_group_layout, glyph);
            }
            for text in texts {
                self.render_eltharn_gui_text(text);
            }
        });
    }
    fn render_eltharn_gui_element(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        element: &GuiElementRenderState,
    ) {
        match element {
            GuiElementRenderState::ColoredRectangle {
                pose,
                x0,
                y0,
                x1,
                y1,
                color1,
                color2,
                scissor,
                pipeline,
                ..
            } => {
                self.apply_eltharn_scissor(*scissor);
                let top = argb_to_rgba(*color1);
                let bottom = argb_to_rgba(*color2);
                let corners: [[f32; 2]; 4] = [
                    transform_point(*pose, *x0 as f32, *y0 as f32).into(),
                    transform_point(*pose, *x0 as f32, *y1 as f32).into(),
                    transform_point(*pose, *x1 as f32, *y1 as f32).into(),
                    transform_point(*pose, *x1 as f32, *y0 as f32).into(),
                ];
                let (x0, y0, x1, y1) = transform_rect(*pose, *x0, *y0, *x1, *y1);
                match pipeline {
                    RenderPipeline::GuiInvert => {
                        self.renderer
                            .draw_invert_rect(x0, y0, x1 - x0, y1 - y0, top)
                    }
                    RenderPipeline::GuiTextHighlight => {
                        self.renderer
                            .draw_additive_rect(x0, y0, x1 - x0, y1 - y0, top)
                    }
                    _ => {
                        self.renderer
                            .draw_color_mesh_quad(corners, [top, bottom, bottom, top]);
                    }
                }
            }
            GuiElementRenderState::ResourceTextureBlit {
                pipeline,
                texture_setup,
                pose,
                x0,
                y0,
                x1,
                y1,
                u0,
                u1,
                v0,
                v1,
                color,
                scissor,
                ..
            } => {
                self.apply_eltharn_scissor(*scissor);
                let Some(texture) = &texture_setup.texture0 else {
                    return;
                };
                let (rx0, ry0, rx1, ry1) = transform_rect(*pose, *x0, *y0, *x1, *y1);
                let texture_idx = self.renderer.get_or_load_custom_texture(
                    device,
                    queue,
                    texture_bind_group_layout,
                    &self.resource_manager,
                    texture,
                );
                match pipeline {
                    RenderPipeline::EndPortal => {
                        let Some(portal_texture) = &texture_setup.texture1 else {
                            return;
                        };
                        let portal_idx = self.renderer.get_or_load_custom_texture(
                            device,
                            queue,
                            texture_bind_group_layout,
                            &self.resource_manager,
                            portal_texture,
                        );
                        self.renderer.draw_end_portal(
                            [rx0, ry0, rx1 - rx0, ry1 - ry0],
                            texture_idx,
                            portal_idx,
                        );
                    }
                    RenderPipeline::Vignette => self.renderer.draw_vignette(
                        [rx0, ry0, rx1 - rx0, ry1 - ry0],
                        texture_idx,
                        argb_to_rgba(*color),
                    ),
                    _ => self.renderer.draw_custom_texture_region(
                        rx0,
                        ry0,
                        rx1 - rx0,
                        ry1 - ry0,
                        *u0,
                        *v0,
                        *u1,
                        *v1,
                        texture_idx,
                        argb_to_rgba(*color),
                    ),
                }
            }
            GuiElementRenderState::Blit {
                texture_setup,
                pose,
                x0,
                y0,
                x1,
                y1,
                u0,
                u1,
                v0,
                v1,
                color,
                scissor,
                ..
            }
            | GuiElementRenderState::TiledBlit {
                texture_setup,
                pose,
                x0,
                y0,
                x1,
                y1,
                u0,
                u1,
                v0,
                v1,
                color,
                scissor,
                ..
            } => {
                self.apply_eltharn_scissor(*scissor);
                let Some(texture) = &texture_setup.texture0 else {
                    return;
                };
                let menu_list_combined_alpha =
                    composite_alpha_over(MENU_BACKGROUND_ALPHA, MENU_LIST_BACKGROUND_ALPHA);
                match texture.path.as_str() {
                    "gui/menu_background" | "gui/inworld_menu_background" => {
                        let (rx0, ry0, rx1, ry1) = transform_rect(*pose, *x0, *y0, *x1, *y1);
                        self.renderer.draw_gradient_rect(
                            rx0,
                            ry0,
                            rx1 - rx0,
                            ry1 - ry0,
                            [0.0, 0.0, 0.0, MENU_BACKGROUND_ALPHA],
                            [0.0, 0.0, 0.0, MENU_BACKGROUND_ALPHA],
                        );
                        return;
                    }
                    "gui/menu_list_background" | "gui/inworld_menu_list_background" => {
                        let (rx0, ry0, rx1, ry1) = transform_rect(*pose, *x0, *y0, *x1, *y1);
                        self.renderer.draw_gradient_rect(
                            rx0,
                            ry0,
                            rx1 - rx0,
                            ry1 - ry0,
                            [0.0, 0.0, 0.0, menu_list_combined_alpha],
                            [0.0, 0.0, 0.0, menu_list_combined_alpha],
                        );
                        return;
                    }
                    _ => {}
                }
                let (rx0, ry0, rx1, ry1) = transform_rect(*pose, *x0, *y0, *x1, *y1);
                let color = argb_to_rgba(*color);
                if self.renderer.gui_atlas.sprites.contains_key(texture) {
                    self.renderer
                        .draw_sprite(rx0, ry0, rx1 - rx0, ry1 - ry0, texture, color);
                } else {
                    let texture_idx = self.renderer.get_or_load_custom_texture(
                        device,
                        queue,
                        texture_bind_group_layout,
                        &self.resource_manager,
                        texture,
                    );
                    self.renderer.draw_custom_texture_region(
                        rx0,
                        ry0,
                        rx1 - rx0,
                        ry1 - ry0,
                        *u0,
                        *v0,
                        *u1,
                        *v1,
                        texture_idx,
                        color,
                    );
                }
            }
        }
    }
    fn draw_text_transformed(
        &mut self,
        text: &GuiTextRenderState,
        offset_x: i32,
        offset_y: i32,
        color: [f32; 4],
        _is_shadow: bool,
    ) {
        let mut x = (text.x + offset_x) as f32;
        let y = (text.y + offset_y) as f32 + self.font.vanilla_ascent();
        for (index, original_ch) in text.text.chars().enumerate() {
            let ch = if text.obfuscated {
                self.font
                    .obfuscated_char(original_ch, gui_text_obfuscation_seed(text, index))
            } else {
                original_ch
            };
            if ch == '\n' {
                continue;
            }
            let glyph = self.font.glyph(ch);
            if glyph.width > 0.0 {
                let lx0 = x + glyph.left;
                let ly0 = y - glyph.ascent;
                let lx1 = lx0 + glyph.width;
                let ly1 = ly0 + glyph.height;
                let p0 = transform_point(text.pose, lx0, ly0);
                let p1 = transform_point(text.pose, lx1, ly0);
                let p2 = transform_point(text.pose, lx1, ly1);
                let p3 = transform_point(text.pose, lx0, ly1);
                let indices_before = self.renderer.indices.len() as u32;
                self.renderer.add_custom_quad(
                    [p0.0, p0.1],
                    [p1.0, p1.1],
                    [p2.0, p2.1],
                    [p3.0, p3.1],
                    [glyph.u0, glyph.v0],
                    [glyph.u1, glyph.v0],
                    [glyph.u1, glyph.v1],
                    [glyph.u0, glyph.v1],
                    color,
                );
                let draw_count = self.renderer.indices.len() as u32 - indices_before;
                if draw_count > 0 {
                    let sheet_idx = glyph.texture_index;
                    let mut merged = false;
                    if let Some(DrawBatch::FontSheet {
                        sheet_idx: s_idx,
                        start_idx: _,
                        count,
                    }) = self.renderer.batches.last_mut()
                    {
                        if *s_idx == sheet_idx {
                            *count += draw_count;
                            merged = true;
                        }
                    }
                    if !merged {
                        self.renderer.batches.push(DrawBatch::FontSheet {
                            sheet_idx,
                            start_idx: indices_before,
                            count: draw_count,
                        });
                    }
                }
            }
            x += glyph.advance;
        }
    }
    fn render_eltharn_gui_text(&mut self, text: &GuiTextRenderState) {
        self.apply_eltharn_scissor(text.scissor);
        let color = argb_to_rgba(text.color);
        let is_complex = (text.pose.m00 - 1.0).abs() > 0.001
            || (text.pose.m11 - 1.0).abs() > 0.001
            || text.pose.m01.abs() > 0.001
            || text.pose.m10.abs() > 0.001;
        if is_complex {
            if text.shadow {
                let shadow_color = [
                    (((color[0].clamp(0.0, 1.0) * 255.0) as u32 * 255 / 1020) as f32) / 255.0,
                    (((color[1].clamp(0.0, 1.0) * 255.0) as u32 * 255 / 1020) as f32) / 255.0,
                    (((color[2].clamp(0.0, 1.0) * 255.0) as u32 * 255 / 1020) as f32) / 255.0,
                    color[3],
                ];
                self.draw_text_transformed(text, 1, 1, shadow_color, true);
            }
            self.draw_text_transformed(text, 0, 0, color, false);
        } else {
            let (x, y) = transform_point(text.pose, text.x as f32, text.y as f32);
            let obfuscated_text;
            let displayed_text = if text.obfuscated {
                obfuscated_text = text
                    .text
                    .chars()
                    .enumerate()
                    .map(|(index, ch)| {
                        self.font
                            .obfuscated_char(ch, gui_text_obfuscation_seed(text, index))
                    })
                    .collect::<String>();
                obfuscated_text.as_str()
            } else {
                text.text.as_str()
            };
            if displayed_text.contains('\u{1b}') || displayed_text.contains('§') {
                crate::ui::chat::chat_component::draw_formatted_chat_text_ex(
                    &mut self.renderer,
                    &self.font,
                    displayed_text,
                    x,
                    y,
                    color[3],
                    1.0,
                    text.shadow,
                );
            } else if text.shadow {
                self.renderer.draw_text_top_with_shadow(
                    &self.font,
                    displayed_text,
                    x,
                    y,
                    color,
                    1.0,
                );
            } else {
                self.renderer
                    .draw_text_top(&self.font, displayed_text, x, y, color, 1.0);
            }
        }
    }
    fn apply_eltharn_scissor(&mut self, scissor: Option<el_gui::ScreenRectangle>) {
        let scale = self.get_gui_scale_factor(self.last_screen_w, self.last_screen_h) as f32;
        let logical_w = self.last_screen_w / scale;
        let logical_h = self.last_screen_h / scale;
        self.renderer.active_scissor =
            scissor.map(|rect| scissor_window_clamped(rect, logical_w, logical_h));
    }
    fn add_recent_chat(&mut self, text: String) {
        self.chat_component.add_recent_chat(text.clone());
        if text.starts_with('/') {
            if self.recent_chat.last() != Some(&text) {
                self.recent_chat.push(text);
                const MAX_PERSISTED_COMMAND_HISTORY: usize = 50;
                if self.recent_chat.len() > MAX_PERSISTED_COMMAND_HISTORY {
                    let overflow = self.recent_chat.len() - MAX_PERSISTED_COMMAND_HISTORY;
                    self.recent_chat.drain(0..overflow);
                }
                self.save_command_history();
            }
        }
    }
    fn load_command_history(&mut self) {
        let commands = std::fs::read_to_string(&self.command_history_path)
            .map(|contents| {
                contents
                    .lines()
                    .rev()
                    .take(50)
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.chat_component.replace_recent_chat(&commands);
        self.recent_chat = commands;
    }
    fn save_command_history(&self) {
        let mut contents = self
            .recent_chat
            .iter()
            .rev()
            .take(50)
            .cloned()
            .collect::<Vec<_>>();
        contents.reverse();
        let contents = if contents.is_empty() {
            String::new()
        } else {
            format!("{}\n", contents.join("\n"))
        };
        if let Err(error) = std::fs::write(&self.command_history_path, contents) {
            log::error!(
                "Failed to write {}: {error}",
                self.command_history_path.display()
            );
        }
    }
    fn submit_chat(&mut self, text: String, player: &mut LocalPlayer) {
        self.add_recent_chat(text.clone());
        if self.connected_chat {
            self.pending_chat_submissions.push(text);
            return;
        }
        if !text.starts_with('/') {
            self.chat_component
                .add_client_system_message(text, self.gui_ticks);
            return;
        }
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        match parts[0] {
            "/give" => self.handle_give_command(&parts, player),
            "/tp" => self.handle_tp_command(&parts, player),
            "/gamemode" => {
                if let Some(mode) = parts.last().and_then(|mode| GameType::by_name(mode)) {
                    self.requested_game_mode = Some(mode);
                    self.hud
                        .chat_messages
                        .push(format!("Set game mode to {}", mode.name()));
                } else {
                    self.hud.chat_messages.push(
                        "Usage: /gamemode <survival|creative|adventure|spectator>".to_string(),
                    );
                }
            }
            "/time" => self.handle_time_command(&parts),
            "/weather" => self.handle_weather_command(&parts),
            "/kill" => {
                player.health = 20.0;
                player.set_position(glam::Vec3::new(0.0, 80.0, 0.0));
                self.hud.chat_messages.push("Killed player".to_string());
            }
            "/help" => {
                self.hud.chat_messages.push(
                    "Commands: /help, /gamemode, /give, /tp, /time, /weather, /kill".to_string(),
                );
            }
            _ => self
                .hud
                .chat_messages
                .push(format!("Unknown command: {}", parts[0])),
        }
    }
    fn handle_give_command(&mut self, parts: &[&str], player: &mut LocalPlayer) {
        if parts.len() < 2 {
            self.hud
                .chat_messages
                .push("Usage: /give [target] <item> [amount]".to_string());
            return;
        }
        let is_target =
            parts[1].starts_with('@') || (parts.len() >= 3 && parts[2].parse::<i32>().is_err());
        let Some(item_name) = parts.get(if is_target { 2 } else { 1 }) else {
            self.hud
                .chat_messages
                .push("Usage: /give [target] <item> [amount]".to_string());
            return;
        };
        let amount = parts
            .get(if is_target { 3 } else { 2 })
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(1);
        let full_item_name = if item_name.contains(':') {
            (*item_name).to_string()
        } else {
            format!("minecraft:{}", item_name)
        };
        let item_loc = ResourceLocation::from_string(&full_item_name);
        let item_registry = el_core::registry::BuiltInRegistries::items();
        let mut found_item = None;
        for i in 0..item_registry.size() {
            let item = item_registry.get_by_id(i);
            if item.location == item_loc {
                found_item = Some(item.clone());
                break;
            }
        }
        let Some(item) = found_item else {
            self.hud
                .chat_messages
                .push(format!("Item not found: {}", item_name));
            return;
        };
        let stack = el_core::item::ItemStack::new(item, amount);
        if let Some(slot) = player
            .inventory
            .iter_mut()
            .take(36)
            .find(|slot| slot.is_empty())
        {
            *slot = stack;
            self.hud
                .chat_messages
                .push(format!("Gave {} {} to player", amount, item_name));
        } else {
            self.hud.chat_messages.push("Inventory full".to_string());
        }
    }
    fn handle_tp_command(&mut self, parts: &[&str], player: &mut LocalPlayer) {
        if parts.len() < 4 {
            self.hud
                .chat_messages
                .push("Usage: /tp [target] <x> <y> <z>".to_string());
            return;
        }
        let has_target = parts.len() >= 5 || (parts.len() == 4 && parts[1].starts_with('@'));
        let offset = if has_target { 2 } else { 1 };
        if parts.len() < offset + 3 {
            self.hud
                .chat_messages
                .push("Usage: /tp [target] <x> <y> <z>".to_string());
            return;
        }
        let parse_coord = |val_str: &str, current: f32| -> Option<f32> {
            if val_str == "~" {
                Some(current)
            } else if let Some(rest) = val_str.strip_prefix('~') {
                rest.parse::<f32>().ok().map(|value| current + value)
            } else {
                val_str.parse::<f32>().ok()
            }
        };
        if let (Some(x), Some(y), Some(z)) = (
            parse_coord(parts[offset], player.position.x),
            parse_coord(parts[offset + 1], player.position.y),
            parse_coord(parts[offset + 2], player.position.z),
        ) {
            player.set_position(glam::Vec3::new(x, y, z));
            self.hud
                .chat_messages
                .push(format!("Teleported to {:.2}, {:.2}, {:.2}", x, y, z));
        } else {
            self.hud
                .chat_messages
                .push("Invalid coordinates".to_string());
        }
    }
    fn handle_time_command(&mut self, parts: &[&str]) {
        if parts.len() < 3 {
            self.hud
                .chat_messages
                .push("Usage: /time <set|add|query> <value>".to_string());
            return;
        }
        match parts[1] {
            "set" => {
                if let Some(ticks) = parse_time_value(parts[2]) {
                    self.requested_time = Some(TimeRequest::Set(ticks));
                    self.hud
                        .chat_messages
                        .push(format!("Set time to {}", parts[2]));
                } else {
                    self.hud
                        .chat_messages
                        .push(format!("Invalid time value: {}", parts[2]));
                }
            }
            "add" => match parts[2].parse::<i64>() {
                Ok(ticks) => {
                    self.requested_time = Some(TimeRequest::Add(ticks));
                    self.hud
                        .chat_messages
                        .push(format!("Added {} ticks to time", ticks));
                }
                Err(_) => self
                    .hud
                    .chat_messages
                    .push(format!("Invalid time value: {}", parts[2])),
            },
            "query" => self
                .hud
                .chat_messages
                .push("Time query needs server-backed feedback".to_string()),
            _ => self
                .hud
                .chat_messages
                .push("Usage: /time <set|add|query> <value>".to_string()),
        }
    }
    fn handle_weather_command(&mut self, parts: &[&str]) {
        if parts.len() < 2 {
            self.hud
                .chat_messages
                .push("Usage: /weather <clear|rain|thunder>".to_string());
            return;
        }
        match parts[1] {
            "clear" => {
                self.requested_weather = Some(WeatherMode::Clear);
                self.hud
                    .chat_messages
                    .push("Set weather to clear".to_string());
            }
            "rain" => {
                self.requested_weather = Some(WeatherMode::Rain);
                self.hud
                    .chat_messages
                    .push("Set weather to rain".to_string());
            }
            "thunder" => {
                self.requested_weather = Some(WeatherMode::Thunder);
                self.hud
                    .chat_messages
                    .push("Set weather to thunder".to_string());
            }
            _ => self
                .hud
                .chat_messages
                .push("Usage: /weather <clear|rain|thunder>".to_string()),
        }
    }
    fn render_chat_input_and_suggestions(
        &mut self,
        device: &Device,
        queue: &Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        logical_w: f32,
        logical_h: f32,
        input: &str,
        in_bed: Option<&mut InBedChatScreen>,
    ) {
        let box_y = logical_h - 14.0;
        let text_y = logical_h - 12.0;
        let input_background = self.hud_background_options().chat_input_background();
        self.renderer
            .draw_rect(2.0, box_y, logical_w - 4.0, 12.0, input_background);
        let cursor = self.chat_cursor.min(input.chars().count());
        let input_width = (logical_w - 4.0).max(1.0);
        self.chat_input_display_pos = chat_input_scroll_to(
            &self.font,
            input,
            cursor,
            self.chat_input_display_pos,
            input_width,
        );
        let (display_start, display_end) =
            chat_input_display_range(&self.font, input, self.chat_input_display_pos, input_width);
        let display_start_byte = byte_index_at_char(input, display_start);
        let display_end_byte = byte_index_at_char(input, display_end);
        let displayed_input = &input[display_start_byte..display_end_byte];
        let displayed_cursor = cursor
            .saturating_sub(display_start)
            .min(displayed_input.chars().count());
        let cursor_byte = byte_index_at_char(displayed_input, displayed_cursor);
        let insert_cursor = cursor < input.chars().count() || input.encode_utf16().count() >= 256;
        let cursor_x = 4.0
            + self
                .font
                .measure_width(&displayed_input[..cursor_byte], 1.0)
                .ceil()
            + if insert_cursor { 0.0 } else { 1.0 };
        let selection = chat_selection_range(self.chat_selection_anchor, cursor);
        if let Some((selection_start, selection_end)) = selection {
            let clipped_start = selection_start.max(display_start).min(display_end);
            let clipped_end = selection_end.max(display_start).min(display_end);
            if clipped_start < clipped_end {
                let selection_start_byte =
                    byte_index_at_char(input, clipped_start) - display_start_byte;
                let selection_end_byte =
                    byte_index_at_char(input, clipped_end) - display_start_byte;
                let selection_x = 3.0
                    + self
                        .font
                        .measure_width(&displayed_input[..selection_start_byte], 1.0)
                        .ceil();
                let selection_width = self
                    .font
                    .measure_width(
                        &displayed_input[selection_start_byte..selection_end_byte],
                        1.0,
                    )
                    .ceil()
                    + 1.0;
                self.renderer.draw_rect(
                    selection_x,
                    text_y - 1.0,
                    selection_width.max(1.0),
                    11.0,
                    [1.0, 1.0, 1.0, 1.0],
                );
            }
        }
        let input_text = if self.chat_is_draft {
            format!(
                "\u{1b}[38;2;170;170;170m\u{1b}[3m{}\u{1b}[m",
                displayed_input
            )
        } else if let Some((selection_start, selection_end)) = selection {
            let clipped_start = selection_start.max(display_start).min(display_end);
            let clipped_end = selection_end.max(display_start).min(display_end);
            if clipped_start < clipped_end {
                let sel_start_b = byte_index_at_char(input, clipped_start) - display_start_byte;
                let sel_end_b = byte_index_at_char(input, clipped_end) - display_start_byte;
                let prefix = &displayed_input[..sel_start_b];
                let selected = &displayed_input[sel_start_b..sel_end_b];
                let suffix = &displayed_input[sel_end_b..];
                format!(
                    "\u{1b}[38;2;224;224;224m{}\u{1b}[38;2;0;0;255m{}\u{1b}[m\u{1b}[38;2;224;224;224m{}\u{1b}[m",
                    prefix, selected, suffix
                )
            } else if let Some(formatted) =
                crate::ui::chat::command_suggestions::format_chat_input_like_vanilla(
                    input,
                    display_start,
                    display_end,
                    self.server_command_root,
                    &self.server_command_nodes,
                )
            {
                formatted
            } else {
                format!("\u{1b}[38;2;224;224;224m{}\u{1b}[m", displayed_input)
            }
        } else if let Some(formatted) =
            crate::ui::chat::command_suggestions::format_chat_input_like_vanilla(
                input,
                display_start,
                display_end,
                self.server_command_root,
                &self.server_command_nodes,
            )
        {
            formatted
        } else {
            format!("\u{1b}[38;2;224;224;224m{}\u{1b}[m", displayed_input)
        };
        crate::ui::chat::chat_component::draw_formatted_chat_text_ex(
            &mut self.renderer,
            &self.font,
            &input_text,
            4.0,
            text_y,
            1.0,
            1.0,
            true,
        );
        if (self.gui_ticks / 6) % 2 == 0 {
            let cursor_color = [224.0 / 255.0, 224.0 / 255.0, 224.0 / 255.0, 1.0];
            if insert_cursor {
                self.renderer
                    .draw_rect(cursor_x, text_y - 1.0, 1.0, 11.0, cursor_color);
            } else {
                self.renderer.draw_text_top_with_shadow(
                    &self.font,
                    "_",
                    cursor_x,
                    text_y,
                    cursor_color,
                    1.0,
                );
            }
        }
        let suggestions =
            if self.chat_suggestions_popup_open && !self.active_chat_suggestions.is_empty() {
                self.active_chat_suggestions.clone()
            } else {
                self.chat_suggestions_for(input)
            };
        let has_open_suggestions = self.chat_suggestions_popup_open && !suggestions.is_empty();
        if has_open_suggestions {
            let suggestion_limit = suggestions.len().min(10);
            let max_suggestion_offset = suggestions.len().saturating_sub(10);
            self.chat_suggestion_offset = self.chat_suggestion_offset.min(max_suggestion_offset);
            let visible_suggestions = suggestions
                .iter()
                .enumerate()
                .skip(self.chat_suggestion_offset)
                .take(10)
                .collect::<Vec<_>>();
            if cursor == input.chars().count() && display_end == input.chars().count() {
                let selected_idx = self.chat_suggestion_index % suggestions.len().max(1);
                if let Some(suggestion) = suggestions.get(selected_idx) {
                    let base = self
                        .chat_suggestion_original_contents
                        .as_deref()
                        .unwrap_or(input);
                    let applied = suggestion.apply(base);
                    if applied.starts_with(input) {
                        let suffix = &applied[input.len()..];
                        self.renderer.draw_text_top_with_shadow(
                            &self.font,
                            suffix,
                            cursor_x - 1.0,
                            text_y,
                            [128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0, 1.0],
                            1.0,
                        );
                    }
                }
            }
            let suggestion_width = suggestions
                .iter()
                .map(|suggestion| self.font.measure_width(&suggestion.display, 1.0))
                .map(f32::ceil)
                .fold(0.0_f32, f32::max);
            let suggestion_x = chat_suggestion_x(
                &self.font,
                input,
                suggestions
                    .first()
                    .map(|suggestion| suggestion.range_start)
                    .unwrap_or(0),
                display_start,
                suggestion_width,
                logical_w,
            );
            let suggestion_box_width = suggestion_width + 1.0;
            let suggestion_y = logical_h - 15.0 - suggestion_limit as f32 * 12.0;
            let has_previous = self.chat_suggestion_offset > 0;
            let has_next = suggestions.len() > self.chat_suggestion_offset + suggestion_limit;
            if has_previous || has_next {
                self.renderer.draw_rect(
                    suggestion_x,
                    suggestion_y - 1.0,
                    suggestion_box_width,
                    1.0,
                    [0.0, 0.0, 0.0, 208.0 / 255.0],
                );
                self.renderer.draw_rect(
                    suggestion_x,
                    suggestion_y + suggestion_limit as f32 * 12.0,
                    suggestion_box_width,
                    1.0,
                    [0.0, 0.0, 0.0, 208.0 / 255.0],
                );
                if has_previous {
                    let mut marker_x = 0.0;
                    while marker_x < suggestion_box_width {
                        self.renderer.draw_rect(
                            suggestion_x + marker_x,
                            suggestion_y - 1.0,
                            1.0,
                            1.0,
                            [1.0, 1.0, 1.0, 1.0],
                        );
                        marker_x += 2.0;
                    }
                }
                if has_next {
                    let mut marker_x = 0.0;
                    while marker_x < suggestion_box_width {
                        self.renderer.draw_rect(
                            suggestion_x + marker_x,
                            suggestion_y + suggestion_limit as f32 * 12.0,
                            1.0,
                            1.0,
                            [1.0, 1.0, 1.0, 1.0],
                        );
                        marker_x += 2.0;
                    }
                }
            }
            let mut hovered_tooltip: Option<&str> = None;
            let mut hovered_suggestion = false;
            for (index, suggestion) in visible_suggestions.iter() {
                let row_y = suggestion_y + (index - self.chat_suggestion_offset) as f32 * 12.0;
                let selected = *index == self.chat_suggestion_index % suggestions.len().max(1);
                let hovered = self.mouse_x > suggestion_x
                    && self.mouse_x < suggestion_x + suggestion_box_width
                    && self.mouse_y > row_y
                    && self.mouse_y < row_y + 12.0;
                if hovered {
                    self.chat_suggestion_index = *index;
                    hovered_suggestion = true;
                }
                self.renderer.draw_rect(
                    suggestion_x,
                    row_y,
                    suggestion_box_width,
                    12.0,
                    [0.0, 0.0, 0.0, 208.0 / 255.0],
                );
                self.renderer.draw_text_top_with_shadow(
                    &self.font,
                    &suggestion.display,
                    suggestion_x + 1.0,
                    row_y + 2.0,
                    if selected {
                        [1.0, 1.0, 0.0, 1.0]
                    } else {
                        [170.0 / 255.0, 170.0 / 255.0, 170.0 / 255.0, 1.0]
                    },
                    1.0,
                );
            }
            if hovered_suggestion {
                hovered_tooltip = suggestions
                    .get(self.chat_suggestion_index % suggestions.len().max(1))
                    .and_then(|suggestion| suggestion.tooltip.as_deref());
                if self.mouse_x > suggestion_x
                    && self.mouse_x < suggestion_x + suggestion_box_width
                    && self.mouse_y > suggestion_y
                    && self.mouse_y < suggestion_y + suggestion_limit as f32 * 12.0
                {
                    self.pending_cursor = el_gui::input::events::CursorType::PointingHand;
                }
            }
            if let Some(tooltip) = hovered_tooltip {
                self.draw_chat_tooltip(tooltip, logical_w, logical_h);
            }
        } else {
            let mut usage_lines = Vec::<(String, [u8; 3])>::new();
            let usage_range_start = input
                .strip_prefix('/')
                .map(|body| {
                    1 + body[..crate::ui::chat::command_suggestions::last_word_index(body)]
                        .chars()
                        .count()
                })
                .unwrap_or(0);
            if input.starts_with('/') {
                if let Some(error) =
                    crate::ui::chat::command_suggestions::command_parse_error_for_input(
                        input,
                        cursor,
                        self.server_command_root,
                        &self.server_command_nodes,
                    )
                {
                    usage_lines.push((error, [170, 170, 170]));
                } else {
                    for (node_index, usage) in
                        crate::ui::chat::command_suggestions::command_usage_for_input(
                            input,
                            cursor,
                            self.server_command_root,
                            &self.server_command_nodes,
                        )
                    {
                        if self
                            .server_command_nodes
                            .get(node_index as usize)
                            .is_some_and(|node| {
                                matches!(
                                    &node.node_type,
                                    crate::ui::domain::CommandNodeType::Argument { .. }
                                )
                            })
                        {
                            usage_lines.push((usage, [170, 170, 170]));
                        }
                    }
                }
                if !self.chat_visibility_allows_system() {
                    usage_lines.push((
                        crate::ui::i18n::translate("chat_screen.commands_not_allowed").into_owned(),
                        [255, 85, 85],
                    ));
                }
            } else if !input.trim().is_empty() && !self.chat_visibility_allows_player() {
                usage_lines.push((
                    crate::ui::i18n::translate("chat_screen.messages_not_allowed").into_owned(),
                    [255, 85, 85],
                ));
            }
            if !usage_lines.is_empty() {
                let usage_width = usage_lines
                    .iter()
                    .map(|(line, _)| self.font.measure_width(line, 1.0).ceil())
                    .fold(0.0_f32, f32::max);
                let usage_x = chat_suggestion_x(
                    &self.font,
                    input,
                    usage_range_start,
                    display_start,
                    usage_width,
                    logical_w,
                );
                for (line_index, (line, color)) in usage_lines.iter().enumerate() {
                    let usage_y = logical_h - 27.0 - line_index as f32 * 12.0;
                    self.renderer.draw_rect(
                        usage_x,
                        usage_y,
                        usage_width + 2.0,
                        12.0,
                        [0.0, 0.0, 0.0, 208.0 / 255.0],
                    );
                    let usage_ansi = format!(
                        "\u{1b}[38;2;{};{};{}m{}\u{1b}[m",
                        color[0], color[1], color[2], line
                    );
                    crate::ui::chat::chat_component::draw_formatted_chat_text_ex(
                        &mut self.renderer,
                        &self.font,
                        &usage_ansi,
                        usage_x + 1.0,
                        usage_y + 2.0,
                        1.0,
                        1.0,
                        true,
                    );
                }
            }
        }
        if let Some(screen) = in_bed {
            let logical_w_i = logical_w.round() as i32;
            let logical_h_i = logical_h.round() as i32;
            if screen.base_ref().width != logical_w_i || screen.base_ref().height != logical_h_i {
                el_gui::framework::screen::screen_init(screen, logical_w_i, logical_h_i);
            }
            let mut graphics = GuiGraphicsExtractor::new(
                logical_w_i,
                logical_h_i,
                self.mouse_x.floor() as i32,
                self.mouse_y.floor() as i32,
            );
            Screen::extract_render_state(
                screen,
                &mut graphics,
                self.mouse_x.floor() as i32,
                self.mouse_y.floor() as i32,
                0.0,
            );
            self.pending_cursor = graphics.pending_cursor();
            let btn_state = graphics.finish();
            self.render_eltharn_gui_state(device, queue, texture_bind_group_layout, &btn_state);
        }
    }
}
fn world_player_display_name_from_profile(
    scoreboard: &crate::ui::scoreboard::Scoreboard,
    profile_name: Option<&str>,
    fallback_name: &str,
) -> String {
    let profile_name = profile_name
        .filter(|name| !name.is_empty())
        .unwrap_or(fallback_name);
    crate::ui::scoreboard::format_name_for_team(
        scoreboard.get_players_team(profile_name),
        profile_name,
    )
}
fn is_server_chat_suggestion_input(input: &str) -> bool {
    input
        .strip_prefix('/')
        .is_some_and(|command| !command.trim().is_empty())
}
fn argb_to_rgba(color: i32) -> [f32; 4] {
    let color = color as u32;
    let r = ((color >> 16) & 0xFF) as f32 / 255.0;
    let g = ((color >> 8) & 0xFF) as f32 / 255.0;
    let b = (color & 0xFF) as f32 / 255.0;
    let a = ((color >> 24) & 0xFF) as f32 / 255.0;
    [r, g, b, a]
}
fn transform_point(matrix: el_math::Matrix3x2f, x: f32, y: f32) -> (f32, f32) {
    (
        x * matrix.m00 + y * matrix.m10 + matrix.m20,
        x * matrix.m01 + y * matrix.m11 + matrix.m21,
    )
}
fn transform_rect(
    matrix: el_math::Matrix3x2f,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
) -> (f32, f32, f32, f32) {
    let points = [
        transform_point(matrix, x0 as f32, y0 as f32),
        transform_point(matrix, x1 as f32, y0 as f32),
        transform_point(matrix, x0 as f32, y1 as f32),
        transform_point(matrix, x1 as f32, y1 as f32),
    ];
    let min_x = points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
    let max_x = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
    let min_y = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
    let max_y = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
    (min_x, min_y, max_x, max_y)
}
fn scissor_window_clamped(
    rect: el_gui::ScreenRectangle,
    logical_w: f32,
    logical_h: f32,
) -> [f32; 4] {
    let left = rect.left() as f32;
    let top = rect.top() as f32;
    let right = (rect.right() as f32).min(logical_w);
    let bottom = (rect.bottom() as f32).min(logical_h);
    [left, top, right.max(left), bottom.max(top)]
}
fn prepare_screen_for_render(screen: &mut impl el_gui::Screen, width: i32, height: i32) {
    let base = screen.base_ref();
    let initialized = base.initialized();
    let size_changed = base.width != width || base.height != height;
    if !initialized {
        el_gui::screen_init(screen, width, height);
    } else if size_changed {
        el_gui::framework::screen::screen_resize(screen, width, height);
    }
}
fn route_migrated_screen(
    screen: &mut impl el_gui::Screen,
    event: &WindowEvent,
    mouse_x: f32,
    mouse_y: f32,
    shift_held: bool,
    control_held: bool,
) -> bool {
    match event {
        WindowEvent::Ime(Ime::Commit(text)) => {
            let mut handled = false;
            for codepoint in text.chars() {
                handled |= screen.char_typed(el_gui::input::events::CharacterEvent::new(codepoint));
            }
            return handled;
        }
        WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
            let caret_position = cursor
                .map(|(start, _)| text.get(..start).unwrap_or(&text).chars().count())
                .unwrap_or(0);
            let preedit = (!text.is_empty()).then(|| {
                el_gui::input::events::PreeditEvent::new(
                    text.clone(),
                    caret_position,
                    vec![text.clone()],
                    0,
                )
            });
            return screen.preedit_updated(preedit);
        }
        WindowEvent::Ime(Ime::Disabled) => return screen.preedit_updated(None),
        WindowEvent::Ime(Ime::Enabled) => return false,
        _ => {}
    }
    let modifiers = screen_events::HeldModifiers {
        shift: shift_held,
        control: control_held,
    };
    if let Some((key_event, state)) = screen_events::key_event(event, modifiers, false) {
        let handled = match state {
            ElementState::Pressed => screen.key_pressed(key_event),
            ElementState::Released => screen.key_released(key_event),
        };
        if handled {
            return true;
        }
    }
    if let Some(codepoint) = screen_events::character_event(event) {
        if screen.char_typed(el_gui::input::events::CharacterEvent::new(codepoint)) {
            return true;
        }
    }
    if let Some(mouse_event) =
        screen_events::mouse_event(event, mouse_x as f64, mouse_y as f64, modifiers)
    {
        return match event {
            WindowEvent::MouseInput { state, .. } => match state {
                ElementState::Pressed => {
                    let double_click = screen.base().take_pending_double_click();
                    screen.mouse_clicked(mouse_event, double_click)
                }
                ElementState::Released => screen.mouse_released(mouse_event),
            },
            _ => false,
        };
    }
    if let Some((scroll_x, scroll_y)) = screen_events::wheel_delta(event) {
        return screen.mouse_scrolled(mouse_x as f64, mouse_y as f64, scroll_x, scroll_y);
    }
    false
}
fn update_held_gui_modifier(
    sides: &mut u8,
    code: KeyCode,
    state: ElementState,
) -> Option<(usize, bool)> {
    let bit = match code {
        KeyCode::ShiftLeft => 0,
        KeyCode::ShiftRight => 1,
        KeyCode::ControlLeft => 2,
        KeyCode::ControlRight => 3,
        KeyCode::AltLeft => 4,
        KeyCode::AltRight => 5,
        _ => return None,
    };
    if state == ElementState::Pressed {
        *sides |= 1 << bit;
    } else {
        *sides &= !(1 << bit);
    }
    let category = bit / 2;
    Some((category, (*sides & (3 << (category * 2))) != 0))
}
fn pressed_key(event: &WindowEvent, key: KeyCode) -> bool {
    matches!(
        event, WindowEvent::KeyboardInput { event, .. } if event.state ==
        ElementState::Pressed && (matches!(event.physical_key, PhysicalKey::Code(code) if
        code == key) || match (key, & event.logical_key) { (KeyCode::Backspace,
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Backspace)) => true,
        (KeyCode::Enter, winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter))
        => true, (KeyCode::NumpadEnter,
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter)) => true,
        (KeyCode::Escape, winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape))
        => true, (KeyCode::Space,
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space)) => true,
        (KeyCode::ArrowLeft,
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowLeft)) => true,
        (KeyCode::ArrowRight,
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::ArrowRight)) => true,
        (KeyCode::Delete, winit::keyboard::Key::Named(winit::keyboard::NamedKey::Delete))
        => true, (KeyCode::Home,
        winit::keyboard::Key::Named(winit::keyboard::NamedKey::Home)) => true,
        (KeyCode::End, winit::keyboard::Key::Named(winit::keyboard::NamedKey::End)) =>
        true, (KeyCode::Tab, winit::keyboard::Key::Named(winit::keyboard::NamedKey::Tab))
        => true, _ => false, })
    )
}
fn byte_index_at_char(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(text.len())
}
fn char_index_at_java_utf16(text: &str, utf16_index: usize) -> usize {
    let mut consumed = 0usize;
    for (char_index, ch) in text.chars().enumerate() {
        if consumed >= utf16_index {
            return char_index;
        }
        consumed = consumed.saturating_add(ch.len_utf16());
        if consumed >= utf16_index {
            return char_index + 1;
        }
    }
    text.chars().count()
}
fn chat_click_action_name(action: &ChatClickAction) -> &'static str {
    match action {
        ChatClickAction::RunCommand { .. } => "run_command",
        ChatClickAction::SuggestCommand { .. } => "suggest_command",
        ChatClickAction::CopyToClipboard { .. } => "copy_to_clipboard",
        ChatClickAction::OpenUrl { .. } => "open_url",
        ChatClickAction::OpenFile { .. } => "open_file",
        ChatClickAction::ChangePage { .. } => "change_page",
        ChatClickAction::ShowDialog => "show_dialog",
        ChatClickAction::Custom { .. } => "custom",
    }
}
pub fn is_allowed_untrusted_chat_url(url: &str) -> bool {
    let url = url.trim();
    let has_http_authority = url
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("http://"))
        || url
            .get(..8)
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case("https://"));
    if !has_http_authority {
        return false;
    }
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    matches!(parsed.scheme(), "http" | "https") && parsed.host_str().is_some()
}
fn open_url_cross_platform(url: &str) {
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    if let Err(error) = result {
        log::warn!("Could not launch the external link handler: {error}");
    }
    #[cfg(target_arch = "wasm32")]
    match web_sys::window().and_then(|window| window.open_with_url(url).ok()) {
        Some(Some(_)) => {}
        Some(None) => log::warn!("Browser blocked opening a server link"),
        None => log::warn!("Could not open a server link in the browser"),
    }
    #[cfg(target_os = "android")]
    if let Err(error) = crate::open_android_url(url) {
        log::warn!("Could not open external link on Android: {error}");
    }
}
fn open_file_cross_platform(path: &str) {
    #[cfg(any(target_os = "linux", target_os = "windows", target_os = "macos"))]
    let target = std::path::Path::new(path);
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(target).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("explorer").arg(target).spawn();
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(target).spawn();
    log::debug!("open_file_cross_platform: {path}");
}
fn chat_selection_range(anchor: Option<usize>, cursor: usize) -> Option<(usize, usize)> {
    let anchor = anchor?;
    (anchor != cursor).then_some((anchor.min(cursor), anchor.max(cursor)))
}
fn chat_text_range(text: &str, start: usize, end: usize) -> &str {
    let start = byte_index_at_char(text, start);
    let end = byte_index_at_char(text, end);
    &text[start..end]
}
fn remove_chat_selection(
    text: &mut String,
    cursor: &mut usize,
    anchor: &mut Option<usize>,
) -> bool {
    let Some((start, end)) = chat_selection_range(*anchor, *cursor) else {
        return false;
    };
    let start_byte = byte_index_at_char(text, start);
    let end_byte = byte_index_at_char(text, end);
    text.replace_range(start_byte..end_byte, "");
    *cursor = start;
    *anchor = None;
    true
}
fn insert_text_at(text: &mut String, char_index: usize, inserted: &str) {
    let byte_index = byte_index_at_char(text, char_index);
    text.insert_str(byte_index, inserted);
}
fn chat_cursor_from_x(font: &FontRenderer, text: &str, x: f32) -> usize {
    let mut current_width = 0.0;
    for (index, ch) in text.chars().enumerate() {
        let glyph_width = font.glyph(ch).advance;
        if current_width + glyph_width > x {
            return index;
        }
        current_width += glyph_width;
    }
    text.chars().count()
}
fn chat_suggestion_x(
    font: &FontRenderer,
    input: &str,
    range_start: usize,
    display_pos: usize,
    suggestion_width: f32,
    logical_width: f32,
) -> f32 {
    let display_start = byte_index_at_char(input, display_pos.min(range_start));
    let prefix_end = byte_index_at_char(input, range_start);
    let input_x = 4.0
        + font
            .measure_width(&input[display_start..prefix_end], 1.0)
            .ceil();
    let max_x = (logical_width - suggestion_width).max(0.0);
    input_x.clamp(0.0, max_x) - 1.0
}
fn sort_server_chat_suggestions(
    suggestions: &mut Vec<crate::ui::chat::ChatSuggestion>,
    input: &str,
    cursor: usize,
) {
    let cursor_byte = byte_index_at_char(input, cursor.min(input.chars().count()));
    let partial_command = &input[..cursor_byte];
    let last_word_byte = partial_command
        .char_indices()
        .filter(|(_, ch)| ch.is_whitespace())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()
        .unwrap_or(0);
    let last_word = partial_command[last_word_byte..].to_lowercase();
    let minecraft_prefix = format!("minecraft:{last_word}");
    let mut matching = Vec::with_capacity(suggestions.len());
    let mut partial = Vec::with_capacity(suggestions.len());
    for suggestion in suggestions.drain(..) {
        let text = &suggestion.replacement;
        if text.starts_with(&last_word) || text.starts_with(&minecraft_prefix) {
            matching.push(suggestion);
        } else {
            partial.push(suggestion);
        }
    }
    matching.extend(partial);
    *suggestions = matching;
}
fn suggestion_offset_for_selection(
    selected: usize,
    suggestion_count: usize,
    current_offset: usize,
) -> usize {
    let max_offset = suggestion_count.saturating_sub(10);
    let first = current_offset.min(max_offset);
    let last = first + 9;
    if selected < first {
        selected.min(max_offset)
    } else if selected > last {
        selected
            .saturating_add(1)
            .saturating_sub(10)
            .min(max_offset)
    } else {
        first
    }
}
fn chat_input_display_range(
    font: &FontRenderer,
    input: &str,
    display_pos: usize,
    max_width: f32,
) -> (usize, usize) {
    let chars = input.chars().collect::<Vec<_>>();
    let display_pos = display_pos.min(chars.len());
    if display_pos == 0 && font.measure_width(input, 1.0) <= max_width {
        return (0, chars.len());
    }
    let start = display_pos;
    let mut width = 0.0;
    let mut end = start;
    while end < chars.len() {
        let glyph_width = font.glyph(chars[end]).advance;
        if width + glyph_width > max_width {
            break;
        }
        end += 1;
        width += glyph_width;
    }
    (start, end)
}
fn chat_input_scroll_to(
    font: &FontRenderer,
    input: &str,
    cursor: usize,
    display_pos: usize,
    max_width: f32,
) -> usize {
    let chars = input.chars().collect::<Vec<_>>();
    let value_len = chars.len();
    let mut display_pos = display_pos.min(value_len);
    let displayed = chat_input_display_range(font, input, display_pos, max_width);
    let mut last_pos = displayed.1;
    if cursor == display_pos {
        let mut width = 0.0;
        let mut rewind = 0usize;
        for index in (0..value_len).rev() {
            let glyph_width = font.glyph(chars[index]).advance;
            if width + glyph_width > max_width {
                break;
            }
            width += glyph_width;
            rewind += 1;
        }
        display_pos = display_pos.saturating_sub(rewind);
        last_pos = chat_input_display_range(font, input, display_pos, max_width).1;
    }
    if cursor > last_pos {
        display_pos = display_pos.saturating_add(cursor - last_pos);
    } else if cursor <= display_pos {
        display_pos = cursor;
    }
    display_pos.min(value_len)
}
fn insert_char_at(text: &mut String, char_index: usize, ch: char) {
    let byte_index = byte_index_at_char(text, char_index);
    text.insert(byte_index, ch);
}
fn remove_char_before(text: &mut String, cursor: &mut usize) {
    if *cursor == 0 {
        return;
    }
    let start = byte_index_at_char(text, *cursor - 1);
    let end = byte_index_at_char(text, *cursor);
    text.replace_range(start..end, "");
    *cursor -= 1;
}
fn remove_char_at(text: &mut String, cursor: usize) {
    if cursor >= text.chars().count() {
        return;
    }
    let start = byte_index_at_char(text, cursor);
    let end = byte_index_at_char(text, cursor + 1);
    text.replace_range(start..end, "");
}
fn delete_word_before(text: &mut String, cursor: &mut usize) {
    let target = chat_word_position(text, -1, *cursor);
    if target < *cursor {
        let start = byte_index_at_char(text, target);
        let end = byte_index_at_char(text, *cursor);
        text.replace_range(start..end, "");
        *cursor = target;
    }
}
fn chat_word_position(text: &str, dir: i32, from: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let length = chars.len();
    let mut result = from.min(length);
    let reverse = dir < 0;
    for _ in 0..dir.unsigned_abs() {
        if !reverse {
            result = match chars[result.min(length)..].iter().position(|&c| c == ' ') {
                Some(offset) => result + offset,
                None => length,
            };
            while result < length && chars[result] == ' ' {
                result += 1;
            }
        } else {
            while result > 0 && chars[result - 1] == ' ' {
                result -= 1;
            }
            while result > 0 && chars[result - 1] != ' ' {
                result -= 1;
            }
        }
    }
    result
}
fn delete_word_after(text: &mut String, cursor: usize) {
    let target = chat_word_position(text, 1, cursor);
    if target > cursor {
        let start = byte_index_at_char(text, cursor);
        let end = byte_index_at_char(text, target);
        text.replace_range(start..end, "");
    }
}
fn java_string_hash(text: &str) -> i32 {
    let mut hash = 0i32;
    for unit in text.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(unit as i32);
    }
    hash
}
fn parse_time_value(value: &str) -> Option<i64> {
    match value {
        "day" => Some(1000),
        "noon" => Some(6000),
        "night" => Some(13000),
        "midnight" => Some(18000),
        _ => value.parse::<i64>().ok(),
    }
}
#[allow(dead_code)]
pub fn get_block_tint_color(item_path: &str) -> [f32; 4] {
    match item_path {
        "grass_block" | "short_grass" | "grass" | "tall_grass" | "fern" | "large_fern" | "bush" => {
            [0.4745, 0.7529, 0.3529, 1.0]
        }
        "oak_leaves" | "jungle_leaves" | "acacia_leaves" | "dark_oak_leaves"
        | "mangrove_leaves" | "vine" => [0.2824, 0.7098, 0.1294, 1.0],
        "spruce_leaves" => [0.3804, 0.6000, 0.3804, 1.0],
        "birch_leaves" => [0.5020, 0.6549, 0.3333, 1.0],
        "leaf_litter" => [0.3608, 0.5569, 0.1961, 1.0],
        _ => [1.0, 1.0, 1.0, 1.0],
    }
}
pub fn is_flat_block(item_path: &str) -> bool {
    if item_path.ends_with("_sapling")
        || item_path.ends_with("_coral")
        || item_path.ends_with("_coral_fan")
        || item_path.ends_with("_torch")
        || item_path.ends_with("_rail")
        || item_path.ends_with("_tulip")
        || item_path.ends_with("_fungus")
        || item_path.ends_with("_roots")
        || item_path.ends_with("_vines")
    {
        return true;
    }
    matches!(
        item_path,
        "torch"
            | "rail"
            | "dandelion"
            | "poppy"
            | "blue_orchid"
            | "allium"
            | "azure_bluet"
            | "red_tulip"
            | "orange_tulip"
            | "white_tulip"
            | "pink_tulip"
            | "oxeye_daisy"
            | "cornflower"
            | "lily_of_the_valley"
            | "wither_rose"
            | "brown_mushroom"
            | "red_mushroom"
            | "crimson_fungus"
            | "warped_fungus"
            | "crimson_roots"
            | "warped_roots"
            | "nether_sprouts"
            | "dead_bush"
            | "short_grass"
            | "grass"
            | "fern"
            | "sugar_cane"
            | "kelp"
            | "seagrass"
            | "tall_seagrass"
            | "vine"
            | "glow_lichen"
            | "ladder"
            | "snow"
            | "fire"
            | "soul_fire"
    )
}
impl GuiManager {
    #[cfg(feature = "extensions")]
    pub(crate) fn refresh_keybind_entries(&mut self, controls: &crate::input::VanillaKeyMappings) {
        if let Some(screen) = &mut self.active_screen {
            refresh_keybind_screen(screen, controls);
        }
        for screen in &mut self.menu_parent_stack {
            refresh_keybind_screen(screen, controls);
        }
    }
}
#[cfg(feature = "extensions")]
fn refresh_keybind_screen(screen: &mut ActiveScreen, controls: &crate::input::VanillaKeyMappings) {
    if let ActiveScreen::KeyBinds {
        screen,
        waiting_for_key,
    } = screen
    {
        let mut entries = build_keybind_entries(controls);
        if let Some(selected) = waiting_for_key {
            if let Some(entry) = entries.iter_mut().find(|entry| entry.name_key == *selected) {
                entry.binding_name = format!("> {} <", entry.binding_name);
            } else {
                *waiting_for_key = None;
            }
        }
        screen.replace_entries(entries);
    }
}
fn build_keybind_entries(
    controls: &crate::input::VanillaKeyMappings,
) -> Vec<el_gui::settings::keybinds::KeyBindEntry> {
    let mappings = controls.mappings();
    let mut entries = Vec::new();
    for (idx, mapping) in mappings.iter().enumerate() {
        let binding_name = crate::input::key_mapping::format_bound_input(mapping.input);
        let is_default = mapping.input == mapping.default_input;
        let category_key = mapping.category_key().to_string();
        entries.push(el_gui::settings::keybinds::KeyBindEntry {
            id: idx,
            name_key: mapping.name.to_string(),
            name_label: mapping.display_label().map(str::to_string),
            category_key,
            category_label: mapping.category_label().map(str::to_string),
            category_order: Some(mapping.category_order()),
            order: mapping.order,
            binding_name,
            is_default,
            is_duplicate: false,
        });
    }
    for i in 0..entries.len() {
        let input_i = mappings[i].input;
        if input_i == crate::input::BoundInput::Unbound {
            continue;
        }
        for j in 0..entries.len() {
            if i != j && mappings[i].conflicts_with(&mappings[j]) {
                entries[i].is_duplicate = true;
                break;
            }
        }
    }
    entries
}
fn wrap_degrees(value: f32) -> f32 {
    let mut wrapped = value % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}
fn mouse_button_id(button: MouseButton) -> Option<u16> {
    match button {
        MouseButton::Left => Some(0),
        MouseButton::Right => Some(1),
        MouseButton::Middle => Some(2),
        MouseButton::Back => Some(3),
        MouseButton::Forward => Some(4),
        MouseButton::Other(id) => Some(id),
    }
}
fn menu_blur_radius_from_stored_option(stored_blur: f32) -> f32 {
    if stored_blur <= 1.0 {
        (stored_blur * 10.0).clamp(0.0, 10.0)
    } else {
        stored_blur.clamp(0.0, 10.0)
    }
}
fn client_information_language_option(value: Option<&str>) -> &str {
    value
        .map(str::trim)
        .filter(|code| {
            !code.is_empty()
                && code.encode_utf16().count() <= 16
                && !code.chars().any(char::is_control)
        })
        .unwrap_or("en_us")
}
fn client_information_allows_listing_option(value: Option<&str>) -> bool {
    value != Some("false")
}
fn pause_external_link_url(submenu: PauseSubmenu) -> Option<&'static str> {
    match submenu {
        PauseSubmenu::ReportBugs => Some("https://aka.ms/snapshotbugs?ref=game"),
        PauseSubmenu::Feedback => Some("https://aka.ms/snapshotfeedback?ref=game"),
        _ => None,
    }
}
fn serverbound_hello_profile_id(
    launcher_profile_id: Option<uuid::Uuid>,
    username: &str,
) -> uuid::Uuid {
    launcher_profile_id.unwrap_or_else(|| el_protocol_backend::offline_uuid(username))
}
fn parse_mouse_sensitivity_option(raw: &str) -> Option<f64> {
    let sensitivity = raw.parse::<f64>().ok()?;
    (sensitivity.is_finite() && (0.0..=1.0).contains(&sensitivity)).then_some(sensitivity)
}
fn format_mouse_sensitivity_option(sensitivity: f64) -> String {
    if sensitivity.is_finite() && (0.0..=1.0).contains(&sensitivity) {
        sensitivity.to_string()
    } else {
        "0.5".to_owned()
    }
}
fn win_screen_cache_phase(screen: &WinScreen) -> u64 {
    screen.scroll().to_bits() as u64
}
