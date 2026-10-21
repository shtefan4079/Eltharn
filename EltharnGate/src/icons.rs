#![allow(dead_code)]

use iced::widget::svg::Handle;

pub const SVG_HOME: &str = include_str!("../assets/icons/home.svg");
pub const SVG_SETTINGS: &str = include_str!("../assets/icons/settings.svg");
pub const SVG_CLOSE: &str = include_str!("../assets/icons/close.svg");
pub const SVG_MINIMIZE: &str = include_str!("../assets/icons/minimize.svg");
pub const SVG_MAXIMIZE: &str = include_str!("../assets/icons/maximize.svg");
pub const SVG_PLAY: &str = include_str!("../assets/icons/play.svg");
pub const SVG_CHECK: &str = include_str!("../assets/icons/check.svg");
pub const SVG_CROSS: &str = include_str!("../assets/icons/cross.svg");
pub const SVG_INFO: &str = include_str!("../assets/icons/info.svg");
pub const SVG_USER: &str = include_str!("../assets/icons/user.svg");
pub const SVG_FOLDER: &str = include_str!("../assets/icons/folder.svg");
pub const SVG_TRASH: &str = include_str!("../assets/icons/trash.svg");
pub const SVG_PLUS: &str = include_str!("../assets/icons/plus.svg");
pub const SVG_DOWNLOAD: &str = include_str!("../assets/icons/download.svg");
pub const SVG_SCROLL: &str = include_str!("../assets/icons/scroll.svg");
pub const SVG_ROCKET: &str = include_str!("../assets/icons/rocket.svg");

pub const SVG_LOGO: &str = include_str!("../icon.svg");

pub fn svg_handle(raw_svg: &'static str) -> Handle {
    Handle::from_memory(raw_svg.as_bytes())
}
