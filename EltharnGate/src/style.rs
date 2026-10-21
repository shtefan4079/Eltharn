#![allow(dead_code)]

use el_launcher_theme::{Rgba, SurfaceVisual};
use iced::widget::{button, container, pick_list, text_input};
use iced::{Background, Border, Color, Shadow, Vector};

const fn iced_color(color: Rgba) -> Color {
    Color::from_rgba(color.r, color.g, color.b, color.a)
}

pub const ACCENT_GREEN: Color = Color::from_rgb(0.14, 0.78, 0.42);
pub const ACCENT_GREEN_HOVER: Color = Color::from_rgb(0.18, 0.88, 0.48);
pub const ACCENT_GREEN_GLOW: Color = Color::from_rgba(0.14, 0.78, 0.42, 0.35);
pub const ACCENT_ORANGE: Color = Color::from_rgb(0.98, 0.48, 0.18);
pub const ACCENT_BLUE: Color = Color::from_rgb(0.24, 0.55, 0.96);
pub const ACCENT_RED: Color = Color::from_rgb(0.95, 0.32, 0.32);

pub const GLASS_BG: Color = Color::from_rgba(0.06, 0.07, 0.10, 0.65);
pub const GLASS_CARD_BG: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.035);
pub const GLASS_CARD_BORDER: Color = Color::from_rgba(1.0, 1.0, 1.0, 0.08);

pub trait TextBoldExt {
    fn bold(self) -> Self;
}

impl TextBoldExt for iced::widget::Text<'_> {
    fn bold(self) -> Self {
        self.font(iced::Font {
            weight: iced::font::Weight::Bold,
            ..iced::Font::default()
        })
    }
}

pub fn horizontal_space(width: impl Into<iced::Length>) -> iced::widget::Space {
    iced::widget::Space::new().width(width)
}

pub fn vertical_space(height: impl Into<iced::Length>) -> iced::widget::Space {
    iced::widget::Space::new().height(height)
}

fn surface_style(visual: SurfaceVisual) -> container::Style {
    container::Style {
        background: Some(Background::Color(iced_color(visual.background))),
        border: Border {
            color: iced_color(visual.border.color),
            width: visual.border.width,
            radius: visual.border.radius.into(),
        },
        shadow: visual.shadow.map_or_else(Shadow::default, |shadow| Shadow {
            color: iced_color(shadow.color),
            offset: Vector::new(shadow.offset_x, shadow.offset_y),
            blur_radius: shadow.blur_radius,
        }),
        text_color: visual.text_color.map(iced_color),
        ..container::Style::default()
    }
}

pub fn glass_panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.05, 0.06, 0.08, 0.40))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            width: 1.0,
            radius: 12.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn card_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(GLASS_CARD_BG)),
        border: Border {
            color: GLASS_CARD_BORDER,
            width: 1.0,
            radius: 10.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn subtle_panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.025))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.06),
            width: 1.0,
            radius: 8.0.into(),
        },
        ..container::Style::default()
    }
}

pub fn status_hero_style(ready: bool) -> container::Style {
    let (bg, border, glow) = if ready {
        (
            Color::from_rgba(0.04, 0.16, 0.09, 0.55),
            Color::from_rgba(0.14, 0.78, 0.42, 0.35),
            Color::from_rgba(0.14, 0.78, 0.42, 0.12),
        )
    } else {
        (
            Color::from_rgba(0.20, 0.05, 0.05, 0.55),
            Color::from_rgba(0.95, 0.32, 0.32, 0.35),
            Color::from_rgba(0.95, 0.32, 0.32, 0.12),
        )
    };

    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: glow,
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}

pub fn play_button_style(status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(ACCENT_GREEN_HOVER)),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: Color::from_rgba(0.18, 0.88, 0.48, 0.50),
                offset: Vector::new(0.0, 3.0),
                blur_radius: 14.0,
            },
            snap: false,
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgb(0.12, 0.68, 0.36))),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.25),
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow::default(),
            snap: false,
        },
        _ => button::Style {
            background: Some(Background::Color(ACCENT_GREEN)),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                ..Default::default()
            },
            shadow: Shadow {
                color: ACCENT_GREEN_GLOW,
                offset: Vector::new(0.0, 2.0),
                blur_radius: 10.0,
            },
            snap: false,
        },
    }
}

pub fn action_button_style(status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.14))),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.25),
                width: 1.0,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.20))),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.35),
                width: 1.0,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.03))),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.20),
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.05),
                width: 1.0,
            },
            shadow: Shadow::default(),
            snap: false,
        },
        _ => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.08))),
            text_color: Color::WHITE,
            border: Border {
                radius: 8.0.into(),
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.12),
                width: 1.0,
            },
            shadow: Shadow::default(),
            snap: false,
        },
    }
}

pub fn outline_button_style(status: button::Status) -> button::Style {
    match status {
        button::Status::Hovered => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.06))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.30),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Pressed => button::Style {
            background: Some(Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.10))),
            text_color: Color::WHITE,
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.40),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        button::Status::Disabled => button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.25),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.08),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
        _ => button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: Color::from_rgba(1.0, 1.0, 1.0, 0.85),
            border: Border {
                color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
                width: 1.0,
                radius: 8.0.into(),
            },
            shadow: Shadow::default(),
            snap: false,
        },
    }
}

pub fn nav_tab_button_style(status: button::Status, active: bool) -> button::Style {
    let (bg, text_color, border_color) = if active {
        (
            Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            Color::WHITE,
            Color::from_rgba(1.0, 1.0, 1.0, 0.20),
        )
    } else {
        match status {
            button::Status::Hovered => (
                Color::from_rgba(1.0, 1.0, 1.0, 0.06),
                Color::WHITE,
                Color::from_rgba(1.0, 1.0, 1.0, 0.10),
            ),
            button::Status::Pressed => (
                Color::from_rgba(1.0, 1.0, 1.0, 0.09),
                Color::WHITE,
                Color::from_rgba(1.0, 1.0, 1.0, 0.15),
            ),
            _ => (
                Color::TRANSPARENT,
                Color::from_rgba(1.0, 1.0, 1.0, 0.60),
                Color::TRANSPARENT,
            ),
        }
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color,
        border: Border {
            radius: 8.0.into(),
            color: border_color,
            width: if active { 1.0 } else { 0.0 },
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn window_control_style(status: button::Status, is_close: bool) -> button::Style {
    let (bg, text_color) = match status {
        button::Status::Hovered => {
            if is_close {
                (Color::from_rgb(0.85, 0.20, 0.20), Color::WHITE)
            } else {
                (Color::from_rgba(1.0, 1.0, 1.0, 0.10), Color::WHITE)
            }
        }
        button::Status::Pressed => {
            if is_close {
                (Color::from_rgb(0.70, 0.15, 0.15), Color::WHITE)
            } else {
                (Color::from_rgba(1.0, 1.0, 1.0, 0.15), Color::WHITE)
            }
        }
        _ => (Color::TRANSPARENT, Color::from_rgba(1.0, 1.0, 1.0, 0.65)),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color,
        border: Border {
            radius: 6.0.into(),
            ..Default::default()
        },
        shadow: Shadow::default(),
        snap: false,
    }
}

pub fn pick_list_style(status: pick_list::Status) -> pick_list::Style {
    let (border_color, bg) = match status {
        pick_list::Status::Hovered => (
            Color::from_rgba(1.0, 1.0, 1.0, 0.25),
            Color::from_rgba(1.0, 1.0, 1.0, 0.08),
        ),
        pick_list::Status::Opened { .. } => (ACCENT_GREEN, Color::from_rgba(1.0, 1.0, 1.0, 0.10)),
        _ => (
            Color::from_rgba(1.0, 1.0, 1.0, 0.12),
            Color::from_rgba(1.0, 1.0, 1.0, 0.04),
        ),
    };

    pick_list::Style {
        text_color: Color::WHITE,
        placeholder_color: Color::from_rgba(1.0, 1.0, 1.0, 0.40),
        handle_color: Color::from_rgba(1.0, 1.0, 1.0, 0.60),
        background: Background::Color(bg),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
    }
}

pub fn custom_menu_style(_theme: &iced::Theme) -> iced::widget::overlay::menu::Style {
    iced::widget::overlay::menu::Style {
        text_color: Color::WHITE,
        background: Background::Color(Color::from_rgb(0.08, 0.09, 0.13)),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.15),
            width: 1.0,
            radius: 8.0.into(),
        },
        selected_text_color: Color::WHITE,
        selected_background: Background::Color(Color::from_rgba(0.14, 0.78, 0.42, 0.25)),
        shadow: Shadow::default(),
    }
}

pub fn text_input_style(status: text_input::Status) -> text_input::Style {
    let border_color = match status {
        text_input::Status::Focused { .. } => ACCENT_GREEN,
        text_input::Status::Hovered => Color::from_rgba(1.0, 1.0, 1.0, 0.25),
        _ => Color::from_rgba(1.0, 1.0, 1.0, 0.12),
    };

    text_input::Style {
        background: Background::Color(Color::from_rgba(1.0, 1.0, 1.0, 0.04)),
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        icon: Color::from_rgba(1.0, 1.0, 1.0, 0.40),
        placeholder: Color::from_rgba(1.0, 1.0, 1.0, 0.35),
        value: Color::WHITE,
        selection: Color::from_rgba(0.14, 0.78, 0.42, 0.30),
    }
}

pub fn avatar_style_for_name(name: &str) -> container::Style {
    let hash: u32 = name.bytes().fold(0, |acc, b| acc.wrapping_add(b as u32));
    let colors = [
        Color::from_rgb(0.20, 0.60, 0.85),
        Color::from_rgb(0.85, 0.45, 0.20),
        Color::from_rgb(0.40, 0.75, 0.30),
        Color::from_rgb(0.70, 0.30, 0.80),
        Color::from_rgb(0.85, 0.70, 0.20),
    ];
    let base = colors[(hash as usize) % colors.len()];

    container::Style {
        background: Some(Background::Color(Color::from_rgba(
            base.r, base.g, base.b, 0.25,
        ))),
        border: Border {
            color: Color::from_rgba(base.r, base.g, base.b, 0.50),
            width: 1.0,
            radius: 8.0.into(),
        },
        text_color: Some(Color::WHITE),
        ..container::Style::default()
    }
}

pub fn dialog_panel_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgb(0.09, 0.10, 0.14))),
        border: Border {
            color: Color::from_rgba(1.0, 1.0, 1.0, 0.14),
            width: 1.0,
            radius: 12.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.60),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 32.0,
        },
        ..container::Style::default()
    }
}

pub fn toast_panel_style(kind: u8) -> container::Style {
    let (bg, border) = match kind {
        1 => (
            Color::from_rgba(0.05, 0.18, 0.09, 0.90),
            Color::from_rgba(0.14, 0.78, 0.42, 0.40),
        ),
        2 => (
            Color::from_rgba(0.22, 0.06, 0.06, 0.90),
            Color::from_rgba(0.95, 0.32, 0.32, 0.40),
        ),
        _ => (
            Color::from_rgba(0.06, 0.12, 0.22, 0.90),
            Color::from_rgba(0.24, 0.55, 0.96, 0.40),
        ),
    };

    container::Style {
        background: Some(Background::Color(bg)),
        border: Border {
            color: border,
            width: 1.0,
            radius: 8.0.into(),
        },
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.40),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 16.0,
        },
        ..container::Style::default()
    }
}
