use crate::icons::{
    SVG_CLOSE, SVG_HOME, SVG_LOGO, SVG_MAXIMIZE, SVG_MINIMIZE, SVG_PLUS, SVG_SETTINGS, svg_handle,
};
use crate::launcher::EltharnGateApp;
use crate::launcher::types::{Message, Page};
use crate::style::{
    TextBoldExt, horizontal_space, nav_tab_button_style, outline_button_style, window_control_style,
};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, column, container, row, stack, svg, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding};

pub fn view_title_bar(app: &EltharnGateApp) -> Element<'_, Message> {
    let radius = app.corner_radius;

    let logo = row![
        svg(svg_handle(SVG_LOGO)).width(22).height(22),
        text("Eltharn Gate").size(14).bold().color(Color::WHITE),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let min_btn = window_button(SVG_MINIMIZE, Message::MinimizeWindow, false);
    let max_btn = window_button(SVG_MAXIMIZE, Message::MaximizeWindow, false);
    let close_btn = window_button(SVG_CLOSE, Message::CloseWindow, true);

    let background_drag =
        iced::widget::mouse_area(container("").width(Length::Fill).height(Length::Fill))
            .on_press(Message::TitleBarPressed);

    let content = container(
        row![
            logo,
            horizontal_space(Length::Fill),
            text(&app.status_message)
                .size(11)
                .color(Color::from_rgba(1.0, 1.0, 1.0, 0.55)),
            horizontal_space(10),
            row![min_btn, max_btn, close_btn]
                .spacing(4)
                .align_y(Alignment::Center),
        ]
        .align_y(Alignment::Center)
        .padding(Padding::new(0.0).right(12.0)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_y(Vertical::Center)
    .padding(Padding::new(0.0).left(14.0).right(6.0));

    container(stack![background_drag, content])
        .width(Length::Fill)
        .height(44)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.20))),
            border: Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: iced::border::Radius::new(0.0)
                    .top_left(radius)
                    .top_right(radius),
            },
            ..container::Style::default()
        })
        .into()
}

pub fn view_top_navigation(app: &EltharnGateApp) -> Element<'_, Message> {
    let nav = row![
        nav_button(app, Page::Eltharn, "Eltharn", SVG_HOME),
        nav_button(app, Page::Settings, "Settings", SVG_SETTINGS),
        horizontal_space(Length::Fill),
        account_badge(app),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    container(nav)
        .width(Length::Fill)
        .padding(Padding::new(6.0).bottom(10.0).left(14.0).right(14.0))
        .style(|_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.15))),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.05),
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

pub fn view_sidebar(app: &EltharnGateApp) -> Element<'_, Message> {
    let radius = app.corner_radius;

    let nav = column![
        sidebar_nav_button(app, Page::Eltharn, "Eltharn", SVG_HOME),
        sidebar_nav_button(app, Page::Settings, "Settings", SVG_SETTINGS),
    ]
    .spacing(8)
    .width(Length::Fill)
    .align_x(Horizontal::Center);

    container(
        column![
            nav,
            iced::widget::Space::new()
                .width(Length::Fill)
                .height(Length::Fill),
            account_badge_vertical(app),
        ]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .width(80)
    .height(Length::Fill)
    .padding(Padding::new(8.0).top(10.0).bottom(12.0))
    .style(move |_| container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.22))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            width: 1.0,
            radius: iced::border::Radius::new(0.0).bottom_left(radius),
        },
        ..Default::default()
    })
    .into()
}

fn window_button(
    icon: &'static str,
    message: Message,
    is_close: bool,
) -> Element<'static, Message> {
    Button::new(
        svg(svg_handle(icon))
            .width(12)
            .height(12)
            .style(|_, _| svg::Style {
                color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
            }),
    )
    .on_press(message)
    .style(move |_, status| window_control_style(status, is_close))
    .padding(8)
    .into()
}

fn nav_button<'a>(
    app: &EltharnGateApp,
    page: Page,
    label: &'a str,
    svg_icon: &'static str,
) -> Element<'a, Message> {
    let is_active = app.active_page == page;

    let icon = svg(svg_handle(svg_icon))
        .width(15)
        .height(15)
        .style(move |_, _| svg::Style {
            color: Some(if is_active {
                Color::WHITE
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.55)
            }),
        });

    Button::new(
        row![icon, text(label).size(13).bold()]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .on_press(Message::PageChanged(page))
    .style(move |_, status| nav_tab_button_style(status, is_active))
    .padding(Padding::new(6.0).left(14.0).right(14.0))
    .into()
}

fn sidebar_nav_button<'a>(
    app: &EltharnGateApp,
    page: Page,
    label: &'a str,
    svg_icon: &'static str,
) -> Element<'a, Message> {
    let is_active = app.active_page == page;

    let icon = svg(svg_handle(svg_icon))
        .width(20)
        .height(20)
        .style(move |_, _| svg::Style {
            color: Some(if is_active {
                Color::WHITE
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.55)
            }),
        });

    Button::new(
        column![
            icon,
            text(label).size(10).bold().color(if is_active {
                Color::WHITE
            } else {
                Color::from_rgba(1.0, 1.0, 1.0, 0.50)
            }),
        ]
        .width(Length::Fill)
        .align_x(Horizontal::Center)
        .spacing(4),
    )
    .on_press(Message::PageChanged(page))
    .style(move |_, status| nav_tab_button_style(status, is_active))
    .padding(Padding::new(8.0).top(10.0).bottom(10.0))
    .width(Length::Fill)
    .into()
}

fn account_badge<'a>(app: &'a EltharnGateApp) -> Element<'a, Message> {
    if app.active_page == Page::Eltharn && app.active_account().is_none() {
        return iced::widget::Space::new().into();
    }
    if let Some(account) = app.active_account() {
        let player = &account.name;
        let avatar = crate::launcher::views::view_player_avatar(app, player, 24);

        Button::new(
            row![
                avatar,
                text(player.clone()).size(12).bold().color(Color::WHITE)
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .on_press(Message::OpenAddAccountModal)
        .padding(Padding::new(4.0).left(8.0).right(12.0))
        .style(|_, status| match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.10))),
                border: Border {
                    radius: 8.0.into(),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
                    width: 1.0,
                },
                shadow: Default::default(),
                snap: false,
                text_color: Color::WHITE,
            },
            _ => iced::widget::button::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
                border: Border {
                    radius: 8.0.into(),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    width: 1.0,
                },
                shadow: Default::default(),
                snap: false,
                text_color: Color::WHITE,
            },
        })
        .into()
    } else {
        Button::new(
            row![
                svg(svg_handle(SVG_PLUS))
                    .width(12)
                    .height(12)
                    .style(|_, _| svg::Style {
                        color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.70))
                    }),
                text("Add account")
                    .size(11)
                    .bold()
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.75)),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(Message::OpenAddAccountModal)
        .padding(Padding::new(5.0).left(10.0).right(12.0))
        .style(|_, status| outline_button_style(status))
        .into()
    }
}

fn account_badge_vertical<'a>(app: &'a EltharnGateApp) -> Element<'a, Message> {
    if app.active_page == Page::Eltharn && app.active_account().is_none() {
        return iced::widget::Space::new().into();
    }
    if let Some(account) = app.active_account() {
        let player = &account.name;
        let avatar = crate::launcher::views::view_player_avatar(app, player, 26);

        let display_name = if player.len() > 8 {
            format!("{}…", &player[..7])
        } else {
            player.clone()
        };

        Button::new(
            column![
                avatar,
                text(display_name)
                    .size(9)
                    .bold()
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.75)),
            ]
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .spacing(3),
        )
        .on_press(Message::OpenAddAccountModal)
        .width(Length::Fill)
        .padding(Padding::new(6.0).top(6.0).bottom(6.0))
        .style(|_, status| match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.10))),
                border: Border {
                    radius: 8.0.into(),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
                    width: 1.0,
                },
                shadow: Default::default(),
                snap: false,
                text_color: Color::WHITE,
            },
            _ => iced::widget::button::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
                border: Border {
                    radius: 8.0.into(),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                    width: 1.0,
                },
                shadow: Default::default(),
                snap: false,
                text_color: Color::WHITE,
            },
        })
        .into()
    } else {
        Button::new(
            column![
                container(
                    svg(svg_handle(SVG_PLUS))
                        .width(12)
                        .height(12)
                        .style(|_, _| svg::Style {
                            color: Some(Color::from_rgba(1.0, 1.0, 1.0, 0.60))
                        })
                )
                .width(26)
                .height(26)
                .align_x(Horizontal::Center)
                .align_y(Vertical::Center)
                .style(|_| container::Style {
                    background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04))),
                    border: Border {
                        radius: 8.0.into(),
                        color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
                        width: 1.0,
                    },
                    ..Default::default()
                }),
                text("Add")
                    .size(9)
                    .bold()
                    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50)),
            ]
            .width(Length::Fill)
            .align_x(Horizontal::Center)
            .spacing(3),
        )
        .on_press(Message::OpenAddAccountModal)
        .width(Length::Fill)
        .padding(Padding::new(6.0).top(6.0).bottom(6.0))
        .style(|_, status| match status {
            iced::widget::button::Status::Hovered => iced::widget::button::Style {
                background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
                border: Border {
                    radius: 8.0.into(),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.20),
                    width: 1.0,
                },
                shadow: Default::default(),
                snap: false,
                text_color: Color::WHITE,
            },
            _ => iced::widget::button::Style {
                background: Some(Background::Color(Color::TRANSPARENT)),
                border: Border {
                    radius: 8.0.into(),
                    color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                    width: 1.0,
                },
                shadow: Default::default(),
                snap: false,
                text_color: Color::WHITE,
            },
        })
        .into()
    }
}
