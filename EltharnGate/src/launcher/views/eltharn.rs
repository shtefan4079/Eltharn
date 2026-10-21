use crate::icons::{SVG_PLAY, SVG_PLUS, SVG_TRASH, svg_handle};
use crate::launcher::EltharnGateApp;
use crate::launcher::types::{Message, NavLayout};
use crate::style::{
    ACCENT_GREEN, TextBoldExt, action_button_style, card_style, custom_menu_style,
    outline_button_style, pick_list_style, play_button_style,
};
use el_launcher_core::{EltharnLaunchPageInput, EltharnStatusTone, build_eltharn_launch_page};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, column, container, pick_list, row, scrollable, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding};

pub fn view_eltharn(app: &EltharnGateApp) -> Element<'_, Message> {
    let page = build_eltharn_launch_page(EltharnLaunchPageInput {
        active_account_name: app.active_account().map(|a| a.name.clone()),
        protocol_options: app.protocol_versions(),
        selected_protocol: app.config.selected_protocol.clone(),
        backend_status: app.backend_status(&app.config.selected_protocol),
    });
    let ready = page.hero.tone == EltharnStatusTone::Ready;
    let status_color = if ready {
        ACCENT_GREEN
    } else {
        Color::from_rgba(1.0, 1.0, 1.0, 0.65)
    };
    let protocol = container(
        column![
            text("Server protocol").size(12).bold().color(Color::WHITE),
            pick_list(
                page.protocol_options,
                Some(app.config.selected_protocol.clone()),
                Message::EltharnProtocolVersionSelected
            )
            .text_size(12)
            .padding([8, 10])
            .width(Length::Fill)
            .style(|_, status| pick_list_style(status))
            .menu_style(custom_menu_style),
        ]
        .spacing(9),
    )
    .padding(14)
    .width(Length::Fill)
    .style(|_| card_style());
    let launch = Button::new(
        row![
            svg(svg_handle(SVG_PLAY))
                .width(12)
                .height(12)
                .style(|_, _| svg::Style {
                    color: Some(Color::WHITE)
                }),
            text(page.launch_button_label).size(13).bold(),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding([12, 18])
    .width(Length::Fill)
    .on_press_maybe(page.hero.launch_enabled.then_some(Message::LaunchEltharn))
    .style(|_, status| play_button_style(status));
    let launch_panel = container(
        column![
            text(page.hero.title).size(13).bold().color(status_color),
            text(page.hero.detail)
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.45)),
            launch,
        ]
        .spacing(10),
    )
    .padding(14)
    .width(Length::Fill)
    .style(|_| card_style());
    let right = column![
        view_account_card(app)
            .width(Length::Fill)
            .height(Length::Fill),
        protocol,
        launch_panel,
    ]
    .spacing(12);
    let sidebar_width = if app.config.nav_layout == NavLayout::Sidebar {
        80
    } else {
        0
    };
    let content_width = app.window_width - sidebar_width - 66;
    let content: Element<Message> = if content_width < 740 || app.window_height < 560 {
        scrollable(
            column![
                container(app.extensions.library_view().map(Message::Extensions)).height(380),
                container(right).height(460),
            ]
            .spacing(14),
        )
        .into()
    } else {
        row![
            container(app.extensions.library_view().map(Message::Extensions))
                .width(Length::FillPortion(10))
                .height(Length::Fill),
            container(right)
                .width(Length::FillPortion(6))
                .height(Length::Fill),
        ]
        .spacing(14)
        .height(Length::Fill)
        .into()
    };
    container(
        column![
            column![
                text("Eltharn").size(22).bold().color(Color::WHITE),
                text("Manage installed content and launch your game.")
                    .size(12)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.48))
            ]
            .spacing(4),
            content,
        ]
        .spacing(16)
        .height(Length::Fill),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .padding(18)
    .into()
}

fn view_account_card(app: &EltharnGateApp) -> container::Container<'_, Message> {
    let has_accounts = !app.accounts.is_empty();
    let active_name = app.active_player();

    let account_content: Element<'_, Message> = if has_accounts {
        let accounts_list: Vec<Element<'_, Message>> =
            app.accounts
                .iter()
                .enumerate()
                .map(|(index, account)| {
                    let is_active = index == app.config.active_account_index;
                    let avatar = crate::launcher::views::view_player_avatar(app, &account.name, 28);

                    let info = column![
                        text(account.name.clone())
                            .size(12)
                            .bold()
                            .color(if is_active {
                                Color::WHITE
                            } else {
                                Color::from_rgba(1.0, 1.0, 1.0, 0.70)
                            }),
                        text(if account.is_microsoft {
                            "Microsoft Account"
                        } else {
                            "Offline Player"
                        })
                        .size(10)
                        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.40)),
                    ]
                    .spacing(1);

                    let action_btn = if is_active {
                        Button::new(text("Active").size(10).bold().color(ACCENT_GREEN))
                            .padding(Padding::new(4.0).left(8.0).right(8.0))
                            .style(|_, _| iced::widget::button::Style {
                                background: Some(Background::Color(Color::from_rgba(
                                    0.14, 0.78, 0.42, 0.12,
                                ))),
                                border: Border {
                                    radius: 4.0.into(),
                                    color: Color::from_rgba(0.14, 0.78, 0.42, 0.30),
                                    width: 1.0,
                                },
                                shadow: Default::default(),
                                snap: false,
                                text_color: ACCENT_GREEN,
                            })
                    } else {
                        Button::new(text("Use").size(10).bold())
                            .on_press(Message::SelectAccount(index))
                            .padding(Padding::new(4.0).left(10.0).right(10.0))
                            .style(|_, status| outline_button_style(status))
                    };

                    let delete_btn =
                        Button::new(svg(svg_handle(SVG_TRASH)).width(11).height(11).style(
                            |_, _| svg::Style {
                                color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
                            },
                        ))
                        .on_press(Message::OpenDeleteAccountConfirmation(index))
                        .padding(Padding::new(5.0).left(7.0).right(7.0))
                        .style(|_, status| outline_button_style(status));

                    container(
                        row![
                            avatar,
                            info,
                            iced::widget::Space::new().width(Length::Fill),
                            action_btn,
                            delete_btn
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .padding(8)
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(Background::Color(if is_active {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.05)
                        } else {
                            Color::from_rgba(1.0, 1.0, 1.0, 0.02)
                        })),
                        border: Border {
                            radius: 6.0.into(),
                            color: if is_active {
                                Color::from_rgba(0.14, 0.78, 0.42, 0.30)
                            } else {
                                Color::from_rgba(1.0, 1.0, 1.0, 0.04)
                            },
                            width: 1.0,
                        },
                        ..Default::default()
                    })
                    .into()
                })
                .collect();

        scrollable(
            iced::widget::Column::with_children(accounts_list)
                .spacing(6)
                .width(Length::Fill),
        )
        .height(Length::Fill)
        .into()
    } else {
        container(
            column![
                iced::widget::Space::new().height(Length::Fill),
                text("Add an offline account to launch Eltharn.")
                    .size(11)
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.40)),
                Button::new(
                    row![
                        svg(svg_handle(SVG_PLUS))
                            .width(12)
                            .height(12)
                            .style(|_, _| svg::Style {
                                color: Some(Color::WHITE)
                            }),
                        text("Add account").size(12).bold(),
                    ]
                    .spacing(6)
                    .align_y(Alignment::Center),
                )
                .on_press(Message::OpenAddAccountModal)
                .padding(Padding::new(8.0).left(16.0).right(16.0))
                .style(|_, status| action_button_style(status)),
                iced::widget::Space::new().height(Length::Fill),
            ]
            .spacing(8)
            .align_x(Horizontal::Center)
            .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .into()
    };

    let status_subtitle = if has_accounts {
        if let Some(player) = active_name {
            format!("Logged in as {player}")
        } else {
            "No account selected".to_string()
        }
    } else {
        "No accounts configured".to_string()
    };

    let mut header = row![
        text("Accounts").size(14).bold().color(Color::WHITE),
        iced::widget::Space::new().width(Length::Fill),
    ]
    .align_y(Alignment::Center);
    if has_accounts {
        header = header.push(
            Button::new(
                row![
                    svg(svg_handle(SVG_PLUS))
                        .width(11)
                        .height(11)
                        .style(|_, _| svg::Style {
                            color: Some(Color::WHITE)
                        }),
                    text("Add").size(11).bold(),
                ]
                .spacing(5)
                .align_y(Alignment::Center),
            )
            .on_press(Message::OpenAddAccountModal)
            .padding(Padding::new(4.0).left(10.0).right(10.0))
            .style(|_, status| action_button_style(status)),
        );
    }

    container(
        column![
            header,
            text(status_subtitle)
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            account_content,
        ]
        .spacing(10),
    )
    .padding(16)
    .style(|_| card_style())
}
