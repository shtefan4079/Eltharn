use crate::icons::{
    SVG_CHECK, SVG_DOWNLOAD, SVG_FOLDER, SVG_ROCKET, SVG_SCROLL, SVG_SETTINGS, svg_handle,
};
use crate::launcher::EltharnGateApp;
use crate::launcher::types::{AssetDownloadState, Message, NavLayout};
use crate::style::{
    ACCENT_BLUE, ACCENT_GREEN, TextBoldExt, action_button_style, card_style, glass_panel_style,
    nav_tab_button_style, outline_button_style, play_button_style, subtle_panel_style,
};
use iced::widget::{Button, column, container, row, scrollable, svg, text};
use iced::{Alignment, Color, Element, Length, Padding};

pub fn view_settings(app: &EltharnGateApp) -> Element<'_, Message> {
    let is_topbar = app.config.nav_layout == NavLayout::TopBar;
    let is_sidebar = app.config.nav_layout == NavLayout::Sidebar;

    let nav_card = container(
        column![
            text("Navigation Layout")
                .size(14)
                .bold()
                .color(Color::WHITE),
            text("Choose how navigation controls are displayed in the window.")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            row![
                Button::new(text("Top bar").size(12).bold())
                    .on_press(Message::NavLayoutChanged(NavLayout::TopBar))
                    .style(move |_, status| nav_tab_button_style(status, is_topbar))
                    .padding(Padding::new(8.0).left(20.0).right(20.0)),
                Button::new(text("Sidebar").size(12).bold())
                    .on_press(Message::NavLayoutChanged(NavLayout::Sidebar))
                    .style(move |_, status| nav_tab_button_style(status, is_sidebar))
                    .padding(Padding::new(8.0).left(20.0).right(20.0)),
            ]
            .spacing(10),
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|_| card_style());

    let data_path_str = app.paths.root.display().to_string();
    let data_card = container(
        column![
            text("Storage & Data Directory")
                .size(14)
                .bold()
                .color(Color::WHITE),
            text("All game assets, binaries, resource packs, configs, and logs are kept in this directory.")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            container(
                text(data_path_str)
                    .size(11)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.75)),
            )
            .width(Length::Fill)
            .padding(10)
            .style(|_| subtle_panel_style()),
            row![
                Button::new(
                    row![
                        svg(svg_handle(SVG_FOLDER))
                            .width(13)
                            .height(13)
                            .style(|_, _| svg::Style {
                                color: Some(Color::WHITE)
                            }),
                        text("Open Data Folder").size(12).bold(),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenDataFolder)
                .style(|_, status| outline_button_style(status))
                .padding(Padding::new(8.0).left(14.0).right(14.0)),
                Button::new(
                    row![
                        svg(svg_handle(SVG_SETTINGS))
                            .width(13)
                            .height(13)
                            .style(|_, _| svg::Style {
                                color: Some(Color::WHITE)
                            }),
                        text("Open Protocols Folder").size(12).bold(),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenProtocolsFolder)
                .style(|_, status| outline_button_style(status))
                .padding(Padding::new(8.0).left(14.0).right(14.0)),
                Button::new(
                    row![
                        svg(svg_handle(SVG_SCROLL))
                            .width(13)
                            .height(13)
                            .style(|_, _| svg::Style {
                                color: Some(Color::WHITE)
                            }),
                        text("Open Logs Folder").size(12).bold(),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenLogsFolder)
                .style(|_, status| outline_button_style(status))
                .padding(Padding::new(8.0).left(14.0).right(14.0)),
            ]
            .spacing(10),
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|_| card_style());

    let asset_download_view = match &app.asset_download_state {
        AssetDownloadState::Idle => {
            let asset_version =
                crate::launcher::executor::ELTHARN_ASSETS_COMPATIBILITY_VERSION.to_owned();
            Button::new(
                row![
                    svg(svg_handle(SVG_DOWNLOAD))
                        .width(13)
                        .height(13)
                        .style(|_, _| svg::Style {
                            color: Some(Color::WHITE)
                        }),
                    text(format!("Download / Verify Assets ({asset_version})"))
                        .size(12)
                        .bold(),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .on_press(Message::StartAssetDownload(asset_version))
            .style(|_, status| action_button_style(status))
            .padding(Padding::new(8.0).left(16.0).right(16.0))
        }
        AssetDownloadState::Downloading {
            completed,
            total,
            status_text,
        } => Button::new(
            text(format!("{status_text} ({completed}/{total})"))
                .size(12)
                .bold()
                .color(ACCENT_BLUE),
        )
        .padding(Padding::new(8.0).left(16.0).right(16.0))
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.24, 0.55, 0.96, 0.15,
            ))),
            border: iced::Border {
                color: Color::from_rgba(0.24, 0.55, 0.96, 0.40),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Default::default(),
            snap: false,
            text_color: ACCENT_BLUE,
        }),
        AssetDownloadState::Completed => Button::new(
            row![
                svg(svg_handle(SVG_CHECK))
                    .width(13)
                    .height(13)
                    .style(|_, _| svg::Style {
                        color: Some(ACCENT_GREEN)
                    }),
                text("Assets verified successfully!")
                    .size(12)
                    .bold()
                    .color(ACCENT_GREEN),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(Padding::new(8.0).left(16.0).right(16.0))
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.14, 0.78, 0.42, 0.15,
            ))),
            border: iced::Border {
                color: Color::from_rgba(0.14, 0.78, 0.42, 0.40),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Default::default(),
            snap: false,
            text_color: ACCENT_GREEN,
        }),
        AssetDownloadState::Failed(err) => Button::new(
            text(format!("Download failed: {err}"))
                .size(11)
                .bold()
                .color(Color::from_rgb(0.95, 0.32, 0.32)),
        )
        .padding(Padding::new(8.0).left(16.0).right(16.0))
        .style(|_, _| iced::widget::button::Style {
            background: Some(iced::Background::Color(Color::from_rgba(
                0.95, 0.32, 0.32, 0.15,
            ))),
            border: iced::Border {
                color: Color::from_rgba(0.95, 0.32, 0.32, 0.40),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Default::default(),
            snap: false,
            text_color: Color::from_rgb(0.95, 0.32, 0.32),
        }),
    };

    let assets_card = container(
        column![
            text("Game Assets & Validation")
                .size(14)
                .bold()
                .color(Color::WHITE),
            text("Download official Minecraft textures, models, and sound files directly from Mojang servers.")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            asset_download_view,
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|_| card_style());

    let system_card = container(
        column![
            text("System Integration")
                .size(14)
                .bold()
                .color(Color::WHITE),
            text("Install EltharnGate to your system application menu and desktop shortcut.")
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            Button::new(
                row![
                    svg(svg_handle(SVG_ROCKET))
                        .width(13)
                        .height(13)
                        .style(|_, _| svg::Style {
                            color: Some(Color::WHITE)
                        }),
                    text("Install EltharnGate to System").size(12).bold(),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .on_press(Message::InstallToSystem)
            .style(|_, status| play_button_style(status))
            .padding(Padding::new(8.0).left(16.0).right(16.0)),
        ]
        .spacing(10),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|_| card_style());

    container(scrollable(
        column![
            column![
                text("Settings").size(22).bold().color(Color::WHITE),
                text("Configure EltharnGate launcher preferences and storage")
                    .size(12)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            ]
            .spacing(4),
            nav_card,
            app.extensions.view().map(Message::Extensions),
            data_card,
            assets_card,
            system_card,
        ]
        .spacing(14)
        .width(Length::Fill),
    ))
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(18)
    .style(|_| glass_panel_style())
    .into()
}
