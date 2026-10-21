pub mod account_dialogs;
pub mod eltharn;
pub mod navigation;
pub mod settings;

use crate::icons::{SVG_CHECK, SVG_CROSS, SVG_INFO, svg_handle};
use crate::launcher::EltharnGateApp;
use crate::launcher::types::{Message, NavLayout, Page};
use crate::style::{horizontal_space, toast_panel_style, vertical_space};
use iced::{Color, Element};

pub fn view_main(app: &EltharnGateApp) -> Element<'_, Message> {
    let body = match app.active_page {
        Page::Eltharn => eltharn::view_eltharn(app),
        Page::Settings => settings::view_settings(app),
    };

    let radius = app.corner_radius;

    let bg_panel = iced::widget::container(vertical_space(0))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .style(move |_| iced::widget::container::Style {
            background: Some(iced::Background::Color(iced::Color::from_rgba(
                0.02, 0.02, 0.03, 0.01,
            ))),
            border: iced::Border {
                radius: radius.into(),
                width: 0.0,
                color: iced::Color::TRANSPARENT,
            },
            ..Default::default()
        });

    let content_panel = if app.config.nav_layout == NavLayout::Sidebar {
        let sidebar = navigation::view_sidebar(app);
        let body_area = iced::widget::container(body)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .padding(iced::Padding::new(14.0));

        iced::widget::container(iced::widget::column![
            navigation::view_title_bar(app),
            iced::widget::row![sidebar, body_area]
                .width(iced::Length::Fill)
                .height(iced::Length::Fill),
        ])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(1)
    } else {
        iced::widget::container(iced::widget::column![
            navigation::view_title_bar(app),
            navigation::view_top_navigation(app),
            iced::widget::container(body)
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .padding(iced::Padding::new(14.0).top(4.0))
        ])
        .width(iced::Length::Fill)
        .height(iced::Length::Fill)
        .padding(1)
    };

    let handle_size = 6.0;

    let top_left = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::NorthWest));

    let top_edge = iced::widget::mouse_area(
        iced::widget::container("")
            .width(iced::Length::Fill)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::North));

    let top_right = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::NorthEast));

    let left_edge = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(iced::Length::Fill),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::West));

    let right_edge = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(iced::Length::Fill),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::East));

    let bottom_left = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::SouthWest));

    let bottom_edge = iced::widget::mouse_area(
        iced::widget::container("")
            .width(iced::Length::Fill)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::South));

    let bottom_right = iced::widget::mouse_area(
        iced::widget::container("")
            .width(handle_size)
            .height(handle_size),
    )
    .on_press(Message::ResizeWindow(iced::window::Direction::SouthEast));

    let resize_overlay = iced::widget::column![
        iced::widget::row![top_left, top_edge, top_right].width(iced::Length::Fill),
        iced::widget::row![left_edge, horizontal_space(iced::Length::Fill), right_edge]
            .width(iced::Length::Fill)
            .height(iced::Length::Fill),
        iced::widget::row![bottom_left, bottom_edge, bottom_right].width(iced::Length::Fill),
    ]
    .width(iced::Length::Fill)
    .height(iced::Length::Fill);

    let mut main_view: Element<'_, Message> = iced::widget::container(iced::widget::stack![
        bg_panel,
        content_panel,
        resize_overlay
    ])
    .width(iced::Length::Fill)
    .height(iced::Length::Fill)
    .clip(true)
    .style(move |_| iced::widget::container::Style {
        border: iced::Border {
            radius: radius.into(),
            color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            width: 1.0,
        },
        ..Default::default()
    })
    .into();

    if app.show_add_account_modal {
        let modal = account_dialogs::view_add_account_modal(app);
        main_view = iced::widget::stack![main_view, modal].into();
    }

    if let Some(index) = app.account_to_delete {
        let modal = account_dialogs::view_delete_account_modal(app, index);
        main_view = iced::widget::stack![main_view, modal].into();
    }

    if let Some((ref msg, kind, expiry_time)) = app.toast {
        let remaining = expiry_time - app.anim_time;
        let alpha = if remaining < 600.0 {
            (remaining / 600.0).max(0.0)
        } else {
            1.0
        };

        let icon_svg_raw = match kind {
            1 => SVG_CHECK,
            2 => SVG_CROSS,
            _ => SVG_INFO,
        };

        let toast_widget = iced::widget::mouse_area(
            iced::widget::container(
                iced::widget::row![
                    iced::widget::svg(svg_handle(icon_svg_raw))
                        .width(14)
                        .height(14)
                        .style(move |_, _| iced::widget::svg::Style {
                            color: Some(Color::from_rgba(1.0, 1.0, 1.0, alpha)),
                        }),
                    iced::widget::text(msg.as_str())
                        .size(12)
                        .color(iced::Color::from_rgba(1.0, 1.0, 1.0, alpha)),
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            )
            .padding(iced::Padding::new(10.0).left(14.0).right(16.0))
            .style(move |_| {
                let mut s = toast_panel_style(kind);
                if let Some(iced::Background::Color(ref mut c)) = s.background {
                    c.a *= alpha;
                }
                s.border.color.a *= alpha;
                s
            }),
        )
        .on_press(Message::DismissToast);

        let toast_overlay = iced::widget::container(iced::widget::column![
            vertical_space(iced::Length::Fill),
            iced::widget::row![horizontal_space(iced::Length::Fill), toast_widget,],
        ])
        .padding(iced::Padding::new(16.0).right(20.0).bottom(20.0))
        .width(iced::Length::Fill)
        .height(iced::Length::Fill);

        main_view = iced::widget::stack![main_view, toast_overlay].into();
    }

    main_view
}

pub fn view_player_avatar<'a>(
    app: &'a EltharnGateApp,
    player_name: &str,
    size: u32,
) -> Element<'a, Message> {
    use crate::style::TextBoldExt;
    if let Some(handle) = app.player_skins.get(player_name) {
        iced::widget::container(iced::widget::image(handle.clone()).width(size).height(size))
            .width(size)
            .height(size)
            .style(|_| iced::widget::container::Style {
                border: iced::Border {
                    radius: 4.0.into(),
                    color: iced::Color::from_rgba(1.0, 1.0, 1.0, 0.15),
                    width: 1.0,
                },
                ..Default::default()
            })
            .into()
    } else {
        let initials = crate::launcher::utils::player_initials(player_name);
        let name_owned = player_name.to_string();
        iced::widget::container(
            iced::widget::text(initials)
                .size((size / 2).max(9))
                .bold()
                .color(iced::Color::WHITE),
        )
        .width(size)
        .height(size)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center)
        .style(move |_| crate::style::avatar_style_for_name(&name_owned))
        .into()
    }
}
