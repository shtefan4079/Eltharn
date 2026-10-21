use crate::launcher::EltharnGateApp;
use crate::launcher::types::Message;
use crate::style::{
    ACCENT_RED, TextBoldExt, dialog_panel_style, outline_button_style, play_button_style,
    text_input_style,
};
use iced::alignment::{Horizontal, Vertical};
use iced::widget::{Button, column, container, row, text, text_input};
use iced::{Alignment, Background, Color, Element, Length, Padding};

pub fn view_add_account_modal(app: &EltharnGateApp) -> Element<'_, Message> {
    let radius = app.corner_radius;
    let title = text("Add Player Profile")
        .size(16)
        .bold()
        .color(Color::WHITE);
    let subtitle = text("Enter an offline player username to use in Eltharn.")
        .size(11)
        .color(Color::from_rgba(1.0, 1.0, 1.0, 0.50));

    let input = text_input("Username (e.g. Steve)", &app.new_account_username)
        .on_input(Message::NewAccountUsernameChanged)
        .on_submit(Message::SubmitAddOfflineAccount)
        .padding(10)
        .size(13)
        .style(|_, status| text_input_style(status));

    let buttons = row![
        Button::new(text("Cancel").size(12).bold())
            .on_press(Message::CloseAddAccountModal)
            .style(|_, status| outline_button_style(status))
            .padding(Padding::new(8.0).left(16.0).right(16.0)),
        iced::widget::Space::new().width(Length::Fill),
        Button::new(text("Add Profile").size(12).bold())
            .on_press(Message::SubmitAddOfflineAccount)
            .style(|_, status| play_button_style(status))
            .padding(Padding::new(8.0).left(20.0).right(20.0)),
    ]
    .align_y(Alignment::Center);

    let modal_content = container(
        column![title, subtitle, input, buttons]
            .spacing(14)
            .width(360),
    )
    .padding(20)
    .style(|_| dialog_panel_style());

    container(modal_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.60))),
            border: iced::Border {
                radius: radius.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

pub fn view_delete_account_modal(app: &EltharnGateApp, index: usize) -> Element<'_, Message> {
    let radius = app.corner_radius;
    let account_name = app
        .accounts
        .get(index)
        .map(|a| a.name.as_str())
        .unwrap_or("Profile");

    let title = text("Delete Profile").size(16).bold().color(Color::WHITE);
    let subtitle = text(format!(
        "Are you sure you want to remove profile '{account_name}' from the launcher?"
    ))
    .size(12)
    .color(Color::from_rgba(1.0, 1.0, 1.0, 0.70));

    let buttons = row![
        Button::new(text("Cancel").size(12).bold())
            .on_press(Message::CancelDeleteAccount)
            .style(|_, status| outline_button_style(status))
            .padding(Padding::new(8.0).left(16.0).right(16.0)),
        iced::widget::Space::new().width(Length::Fill),
        Button::new(text("Delete").size(12).bold().color(Color::WHITE))
            .on_press(Message::ConfirmDeleteAccount)
            .style(|_, _| iced::widget::button::Style {
                background: Some(Background::Color(ACCENT_RED)),
                text_color: Color::WHITE,
                border: iced::Border {
                    radius: 8.0.into(),
                    ..Default::default()
                },
                shadow: Default::default(),
                snap: false,
            })
            .padding(Padding::new(8.0).left(20.0).right(20.0)),
    ]
    .align_y(Alignment::Center);

    let modal_content = container(column![title, subtitle, buttons].spacing(16).width(360))
        .padding(20)
        .style(|_| dialog_panel_style());

    container(modal_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Horizontal::Center)
        .align_y(Vertical::Center)
        .style(move |_| container::Style {
            background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.60))),
            border: iced::Border {
                radius: radius.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
