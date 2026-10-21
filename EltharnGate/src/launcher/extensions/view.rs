use super::{
    CatalogTab, ExtMessage, ExtPanel,
    catalog::{self, PackageKind, PackageStatus},
};
use crate::icons::{SVG_FOLDER, svg_handle};
use crate::style::{self, TextBoldExt};
use el_extension_manager::package::PackageInfo;
use el_launcher_core::extensions::ExtensionProfile;
use iced::widget::{
    Button, Space, column, container, row, scrollable, svg, text, text_input, toggler,
};
use iced::{Alignment, Color, Element, Length};

const MUTED: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.48);
const SECONDARY: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.70);

fn switch_style(
    theme: &iced::Theme,
    status: iced::widget::toggler::Status,
) -> iced::widget::toggler::Style {
    let mut appearance = iced::widget::toggler::default(theme, status);
    if matches!(
        status,
        iced::widget::toggler::Status::Active { is_toggled: true }
            | iced::widget::toggler::Status::Hovered { is_toggled: true }
    ) {
        appearance.background = iced::Background::Color(style::ACCENT_GREEN);
        appearance.foreground = iced::Background::Color(Color::WHITE);
    }
    appearance
}

fn active(panel: &ExtPanel) -> Option<&ExtensionProfile> {
    panel
        .prefs
        .as_ref()
        .and_then(|prefs| prefs.profiles.iter().find(|p| p.id == prefs.active_profile))
}

fn preview(value: &str, limit: usize) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let text: String = normalized.chars().take(limit).collect();
    if normalized.chars().count() > limit {
        format!("{text}…")
    } else {
        text
    }
}

pub(super) fn management(panel: &ExtPanel) -> Element<'_, ExtMessage> {
    let profile = active(panel);
    let mods_on = profile.is_some_and(|p| p.flags.mods_enabled);
    let shaders_on = profile.is_some_and(|p| p.flags.shaders_enabled);
    container(
        column![
            text("Game extensions").size(14).bold().color(Color::WHITE),
            text("Use mods and shader packs at the next launch.")
                .size(11)
                .color(MUTED),
            row![
                text("Enable mods")
                    .size(12)
                    .color(SECONDARY)
                    .width(Length::Fill),
                toggler(mods_on)
                    .size(20)
                    .style(switch_style)
                    .on_toggle_maybe((!panel.busy).then_some(ExtMessage::Mods)),
            ]
            .align_y(Alignment::Center),
            row![
                text("Enable shader packs")
                    .size(12)
                    .color(SECONDARY)
                    .width(Length::Fill),
                toggler(shaders_on)
                    .size(20)
                    .style(switch_style)
                    .on_toggle_maybe((!panel.busy).then_some(ExtMessage::Shaders)),
            ]
            .align_y(Alignment::Center),
        ]
        .spacing(12),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|_| style::card_style())
    .into()
}

impl ExtPanel {
    pub fn library_view(&self) -> Element<'_, ExtMessage> {
        library(self)
    }
}

fn library(panel: &ExtPanel) -> Element<'_, ExtMessage> {
    let profile = active(panel);
    let kind = panel.tab.kind();
    let installed = catalog::entries(&panel.catalog, kind).count();
    let tabs: Vec<Element<ExtMessage>> = [
        (CatalogTab::Mods, "Mods"),
        (CatalogTab::ShaderPacks, "Shader Packs"),
    ]
    .into_iter()
    .map(|(tab, label)| {
        let selected = panel.tab == tab;
        Button::new(text(label).size(12).bold())
            .padding([9, 15])
            .on_press(ExtMessage::Tab(tab))
            .style(move |_, status| {
                if selected {
                    style::action_button_style(status)
                } else {
                    style::outline_button_style(status)
                }
            })
            .into()
    })
    .collect();
    let folder = Button::new(
        row![
            svg(svg_handle(SVG_FOLDER))
                .width(12)
                .height(12)
                .style(|_, _| svg::Style {
                    color: Some(SECONDARY)
                }),
            text("Open folder").size(12),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
    )
    .padding([9, 12])
    .on_press(ExtMessage::OpenFolder(panel.tab))
    .style(|_, status| style::outline_button_style(status));
    let mut content = column![
        row![
            text("Installed content")
                .size(16)
                .bold()
                .color(Color::WHITE),
            Space::new().width(Length::Fill),
            text(if panel.busy {
                "Reading files…".into()
            } else {
                format!("{installed} installed")
            })
            .size(11)
            .color(MUTED)
        ]
        .align_y(Alignment::Center),
        row![
            row(tabs).spacing(6),
            Space::new().width(Length::Fill),
            folder
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text_input("Search installed files…", &panel.search)
            .on_input(ExtMessage::Search)
            .padding([10, 12])
            .size(12)
            .style(|_, status| style::text_input_style(status)),
    ]
    .spacing(14);
    let enabled = match kind {
        PackageKind::Mod => profile.is_some_and(|p| p.flags.mods_enabled),
        PackageKind::Shaderpack => profile.is_some_and(|p| p.flags.shaders_enabled),
    };
    if !enabled {
        content = content.push(
            text("Disabled in Settings. Your files stay installed.")
                .size(11)
                .color(MUTED),
        );
    }
    let matching = catalog::filtered(&panel.catalog, kind, &panel.search);
    let list: Element<ExtMessage> = if matching.is_empty() {
        container(
            column![
                text(if panel.search.trim().is_empty() {
                    match kind {
                        PackageKind::Mod => "No mods installed",
                        PackageKind::Shaderpack => "No shader packs installed",
                    }
                } else {
                    "No matching files"
                })
                .size(14)
                .bold()
                .color(SECONDARY),
                text(if panel.search.trim().is_empty() {
                    "Open the folder and place Eltharn ZIP packages there."
                } else {
                    "Try another name or clear the search."
                })
                .size(11)
                .color(MUTED),
            ]
            .spacing(8)
            .align_x(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
    } else {
        let rows = matching
            .into_iter()
            .map(|(index, info)| package_row(index, info, profile, !panel.busy))
            .collect::<Vec<_>>();
        scrollable(column(rows).spacing(6))
            .height(Length::Fill)
            .into()
    };
    content = content.push(list);
    if let Some(error) = &panel.error {
        content = content.push(text(error.clone()).size(11).color(style::ACCENT_RED));
    }
    container(content)
        .padding(16)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| style::glass_panel_style())
        .into()
}

fn package_row<'a>(
    index: usize,
    info: &'a PackageInfo,
    profile: Option<&ExtensionProfile>,
    editable: bool,
) -> Element<'a, ExtMessage> {
    let status = catalog::status(info, profile);
    let selected = status.selected();
    let control: Element<ExtMessage> = match PackageKind::of(info) {
        Some(PackageKind::Mod) => toggler(selected)
            .size(18)
            .style(switch_style)
            .on_toggle_maybe(editable.then_some(move |on| ExtMessage::ToggleMod(index, on)))
            .into(),
        Some(PackageKind::Shaderpack) => {
            Button::new(text(if selected { "Selected" } else { "Use" }).size(11))
                .padding([6, 10])
                .on_press_maybe(editable.then_some(if selected {
                    ExtMessage::ClearShader
                } else {
                    ExtMessage::SelectShader(index)
                }))
                .style(|_, status| style::outline_button_style(status))
                .into()
        }
        None => Space::new().into(),
    };
    container(
        row![
            column![
                text(preview(&info.name, 70))
                    .size(12)
                    .bold()
                    .color(Color::WHITE),
                text(info.pin.version.to_string()).size(10).color(MUTED)
            ]
            .spacing(3)
            .width(Length::Fill),
            text(status.badge())
                .size(10)
                .color(if status == PackageStatus::EnabledNextLaunch {
                    style::ACCENT_GREEN
                } else {
                    MUTED
                }),
            control,
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding([10, 12])
    .width(Length::Fill)
    .style(|_| style::subtle_panel_style())
    .into()
}
