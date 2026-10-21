#![forbid(unsafe_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}
impl Rgba {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub color: Rgba,
    pub width: f32,
    pub radius: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowSpec {
    pub color: Rgba,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceVisual {
    pub background: Rgba,
    pub border: Stroke,
    pub shadow: Option<ShadowSpec>,
    pub text_color: Option<Rgba>,
}
pub const WHITE: Rgba = Rgba::new(1.0, 1.0, 1.0, 1.0);
pub const TRANSPARENT: Rgba = Rgba::new(0.0, 0.0, 0.0, 0.0);
pub const BLACK: Rgba = Rgba::new(0.0, 0.0, 0.0, 1.0);
pub const ACCENT_ORANGE: Rgba = Rgba::new(1.0, 0.36, 0.20, 1.0);
pub const ACCENT_GREEN: Rgba = Rgba::new(0.31, 0.86, 0.34, 1.0);
pub const ACCENT_BLUE: Rgba = Rgba::new(0.30, 0.62, 1.0, 1.0);
pub const GLASS_EDGE: Rgba = Rgba::new(1.0, 1.0, 1.0, 0.10);
pub const SHELL_BACKGROUND: Rgba = Rgba::new(0.02, 0.02, 0.03, 0.01);
pub const SHELL_EDGE: Rgba = Rgba::new(1.0, 1.0, 1.0, 0.12);
pub const GLASS_PANEL: SurfaceVisual = SurfaceVisual {
    background: Rgba::new(1.0, 1.0, 1.0, 0.060),
    border: Stroke {
        color: GLASS_EDGE,
        width: 1.0,
        radius: 8.0,
    },
    shadow: Some(ShadowSpec {
        color: Rgba::new(0.0, 0.0, 0.0, 0.16),
        offset_x: 0.0,
        offset_y: 8.0,
        blur_radius: 18.0,
    }),
    text_color: Some(WHITE),
};
pub const SUBTLE_PANEL: SurfaceVisual = SurfaceVisual {
    background: Rgba::new(0.0, 0.0, 0.0, 0.20),
    border: Stroke {
        color: Rgba::new(1.0, 1.0, 1.0, 0.07),
        width: 1.0,
        radius: 8.0,
    },
    shadow: None,
    text_color: Some(WHITE),
};
pub const CARD: SurfaceVisual = SurfaceVisual {
    background: Rgba::new(1.0, 1.0, 1.0, 0.042),
    border: Stroke {
        color: Rgba::new(1.0, 1.0, 1.0, 0.09),
        width: 1.0,
        radius: 12.0,
    },
    shadow: Some(ShadowSpec {
        color: Rgba::new(0.0, 0.0, 0.0, 0.22),
        offset_x: 0.0,
        offset_y: 4.0,
        blur_radius: 16.0,
    }),
    text_color: Some(WHITE),
};
pub const INSET_PANEL: SurfaceVisual = SurfaceVisual {
    background: Rgba::new(0.0, 0.0, 0.0, 0.28),
    border: Stroke {
        color: Rgba::new(1.0, 1.0, 1.0, 0.08),
        width: 1.0,
        radius: 8.0,
    },
    shadow: None,
    text_color: Some(WHITE),
};
pub const DIALOG_PANEL: SurfaceVisual = SurfaceVisual {
    background: Rgba::new(0.08, 0.08, 0.11, 0.97),
    border: Stroke {
        color: Rgba::new(1.0, 1.0, 1.0, 0.13),
        width: 1.0,
        radius: 14.0,
    },
    shadow: Some(ShadowSpec {
        color: Rgba::new(0.0, 0.0, 0.0, 0.55),
        offset_x: 0.0,
        offset_y: 16.0,
        blur_radius: 40.0,
    }),
    text_color: Some(WHITE),
};
pub const SIDEBAR_NAV_SURFACE: SurfaceVisual = SurfaceVisual {
    background: Rgba::new(0.0, 0.0, 0.0, 0.22),
    border: Stroke {
        color: Rgba::new(1.0, 1.0, 1.0, 0.07),
        width: 0.0,
        radius: 0.0,
    },
    shadow: None,
    text_color: None,
};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellMetrics {
    pub baseline_width: f32,
    pub baseline_height: f32,
    pub outer_inset: f32,
    pub outer_radius: f32,
    pub resize_handle_width: f32,
    pub title_bar_height: f32,
    pub top_navigation_top: f32,
    pub top_navigation_bottom: f32,
    pub top_navigation_horizontal: f32,
    pub body_padding: f32,
    pub topbar_body_gap: f32,
    pub sidebar_width: f32,
    pub sidebar_top_padding: f32,
    pub sidebar_horizontal_padding: f32,
    pub sidebar_bottom_padding: f32,
}
pub const DESKTOP_SHELL_METRICS: ShellMetrics = ShellMetrics {
    baseline_width: 950.0,
    baseline_height: 600.0,
    outer_inset: 1.0,
    outer_radius: 16.0,
    resize_handle_width: 6.0,
    title_bar_height: 48.0,
    top_navigation_top: 8.0,
    top_navigation_bottom: 12.0,
    top_navigation_horizontal: 14.0,
    body_padding: 14.0,
    topbar_body_gap: 14.0,
    sidebar_width: 76.0,
    sidebar_top_padding: 4.0,
    sidebar_horizontal_padding: 8.0,
    sidebar_bottom_padding: 12.0,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EltharnLaunchLayout {
    pub outer_padding: u16,
    pub section_gap: u16,
    pub hero_padding: u16,
    pub hero_gap: u16,
    pub card_padding: u16,
    pub split_gap: u16,
    pub protocol_portion: u16,
    pub accounts_portion: u16,
}
pub const ELTHARN_LAUNCH_LAYOUT: EltharnLaunchLayout = EltharnLaunchLayout {
    outer_padding: 18,
    section_gap: 16,
    hero_padding: 18,
    hero_gap: 14,
    card_padding: 16,
    split_gap: 14,
    protocol_portion: 6,
    accounts_portion: 4,
};
pub const MIN_PROTOCOL_CARD_WIDTH: f32 = 420.0;
pub const MIN_ACCOUNTS_CARD_WIDTH: f32 = 280.0;
pub const MIN_TOUCH_TARGET_HEIGHT: f32 = 48.0;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchPageArrangement {
    SplitColumns,
    Stacked,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResponsiveLaunchLayout {
    pub arrangement: LaunchPageArrangement,
    pub outer_padding: f32,
    pub section_gap: f32,
    pub card_padding: f32,
    pub protocol_width: f32,
    pub accounts_width: f32,
    pub minimum_control_height: f32,
}
pub fn responsive_launch_layout(viewport: Viewport) -> ResponsiveLaunchLayout {
    let outer_padding = f32::from(ELTHARN_LAUNCH_LAYOUT.outer_padding);
    let section_gap = f32::from(ELTHARN_LAUNCH_LAYOUT.split_gap);
    let available_width = (viewport.width - outer_padding * 2.0).max(0.0);
    let split_content_width = (available_width - section_gap).max(0.0);
    let portions =
        f32::from(ELTHARN_LAUNCH_LAYOUT.protocol_portion + ELTHARN_LAUNCH_LAYOUT.accounts_portion);
    let protocol_width =
        split_content_width * f32::from(ELTHARN_LAUNCH_LAYOUT.protocol_portion) / portions;
    let accounts_width =
        split_content_width * f32::from(ELTHARN_LAUNCH_LAYOUT.accounts_portion) / portions;
    if protocol_width >= MIN_PROTOCOL_CARD_WIDTH && accounts_width >= MIN_ACCOUNTS_CARD_WIDTH {
        ResponsiveLaunchLayout {
            arrangement: LaunchPageArrangement::SplitColumns,
            outer_padding,
            section_gap,
            card_padding: f32::from(ELTHARN_LAUNCH_LAYOUT.card_padding),
            protocol_width,
            accounts_width,
            minimum_control_height: MIN_TOUCH_TARGET_HEIGHT,
        }
    } else {
        ResponsiveLaunchLayout {
            arrangement: LaunchPageArrangement::Stacked,
            outer_padding,
            section_gap: f32::from(ELTHARN_LAUNCH_LAYOUT.section_gap),
            card_padding: f32::from(ELTHARN_LAUNCH_LAYOUT.card_padding),
            protocol_width: available_width,
            accounts_width: available_width,
            minimum_control_height: MIN_TOUCH_TARGET_HEIGHT,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
}
pub const ACCOUNT_CONTEXT_MENU_WIDTH_CLAMP: f32 = 170.0;
pub const ACCOUNT_CONTEXT_MENU_Y_OFFSET: f32 = 105.0;
pub const ACCOUNT_CONTEXT_MENU_HEIGHT: f32 = 60.0;
pub const INSTANCE_CONTEXT_MENU_WIDTH_CLAMP: f32 = 210.0;
pub const INSTANCE_CONTEXT_MENU_Y_OFFSET: f32 = 140.0;
pub const INSTANCE_CONTEXT_MENU_HEIGHT: f32 = 130.0;
pub fn account_context_menu_position(pointer: Point, viewport: Viewport) -> Point {
    let mut x = pointer.x;
    let mut y = (pointer.y - ACCOUNT_CONTEXT_MENU_Y_OFFSET).max(0.0);
    if x + ACCOUNT_CONTEXT_MENU_WIDTH_CLAMP > viewport.width {
        x = (viewport.width - ACCOUNT_CONTEXT_MENU_WIDTH_CLAMP).max(0.0);
    }
    let max_y =
        (viewport.height - ACCOUNT_CONTEXT_MENU_Y_OFFSET - ACCOUNT_CONTEXT_MENU_HEIGHT).max(0.0);
    y = y.min(max_y);
    Point { x, y }
}
pub fn instance_context_menu_position(pointer: Point, viewport: Viewport) -> Point {
    let mut x = pointer.x;
    let mut y = (pointer.y - INSTANCE_CONTEXT_MENU_Y_OFFSET).max(0.0);
    if x + INSTANCE_CONTEXT_MENU_WIDTH_CLAMP > viewport.width {
        x = (viewport.width - INSTANCE_CONTEXT_MENU_WIDTH_CLAMP).max(0.0);
    }
    let max_y =
        (viewport.height - INSTANCE_CONTEXT_MENU_Y_OFFSET - INSTANCE_CONTEXT_MENU_HEIGHT).max(0.0);
    y = y.min(max_y);
    Point { x, y }
}
