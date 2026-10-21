#![allow(dead_code)]
use crate::ui::chat::chat_component::{WorldTextGlyph, ansi_world_glyphs};
use crate::ui::gui::font::FontRenderer;
use glam::{Mat4, Quat, Vec3};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum BillboardConstraints {
    #[default]
    Fixed = 0,
    Vertical = 1,
    Horizontal = 2,
    Center = 3,
}
impl BillboardConstraints {
    pub fn from_id(id: u8) -> Self {
        match id {
            1 => Self::Vertical,
            2 => Self::Horizontal,
            3 => Self::Center,
            _ => Self::Fixed,
        }
    }
    pub fn compute_orientation(
        self,
        entity_yaw_deg: f32,
        entity_pitch_deg: f32,
        camera_yaw_deg: f32,
        camera_pitch_deg: f32,
    ) -> Quat {
        let (y_deg, x_deg) = match self {
            Self::Fixed => (-entity_yaw_deg, entity_pitch_deg),
            Self::Horizontal => (-entity_yaw_deg, -camera_pitch_deg),
            Self::Vertical => (180.0 - camera_yaw_deg, entity_pitch_deg),
            Self::Center => (180.0 - camera_yaw_deg, -camera_pitch_deg),
        };
        let quat_y = Quat::from_rotation_y(y_deg.to_radians());
        let quat_x = Quat::from_rotation_x(x_deg.to_radians());
        quat_y * quat_x
    }
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DisplayTransformation {
    pub translation: Vec3,
    pub left_rotation: Quat,
    pub scale: Vec3,
    pub right_rotation: Quat,
}
impl Default for DisplayTransformation {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            left_rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
            right_rotation: Quat::IDENTITY,
        }
    }
}
impl DisplayTransformation {
    #[inline]
    pub fn to_mat4(&self) -> Mat4 {
        Mat4::from_translation(self.translation)
            * Mat4::from_quat(self.left_rotation)
            * Mat4::from_scale(self.scale)
            * Mat4::from_quat(self.right_rotation)
    }
    #[allow(dead_code)]
    pub fn slerp(&self, target: &Self, progress: f32) -> Self {
        Self {
            translation: self.translation.lerp(target.translation, progress),
            left_rotation: self.left_rotation.slerp(target.left_rotation, progress),
            scale: self.scale.lerp(target.scale, progress),
            right_rotation: self.right_rotation.slerp(target.right_rotation, progress),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextDisplayAlignment {
    #[default]
    Center,
    Left,
    Right,
}
impl TextDisplayAlignment {
    pub fn from_flags(flags: u8) -> Self {
        if (flags & 8) != 0 {
            Self::Left
        } else if (flags & 16) != 0 {
            Self::Right
        } else {
            Self::Center
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct TextDisplayState {
    pub start_interpolation: i32,
    pub interpolation_duration: i32,
    pub teleport_duration: i32,
    pub transformation: DisplayTransformation,
    pub billboard: BillboardConstraints,
    pub brightness_override: i32,
    pub view_range: f32,
    pub shadow_radius: f32,
    pub shadow_strength: f32,
    pub width: f32,
    pub height: f32,
    pub glow_color_override: i32,
    pub text: String,
    pub line_width: i32,
    pub background_color: i32,
    pub text_opacity: i8,
    pub flags: u8,
}
impl Default for TextDisplayState {
    fn default() -> Self {
        Self {
            start_interpolation: 0,
            interpolation_duration: 0,
            teleport_duration: 0,
            transformation: DisplayTransformation::default(),
            billboard: BillboardConstraints::default(),
            brightness_override: -1,
            view_range: 1.0,
            shadow_radius: 0.0,
            shadow_strength: 1.0,
            width: 0.0,
            height: 0.0,
            glow_color_override: -1,
            text: String::new(),
            line_width: 200,
            background_color: 1073741824,
            text_opacity: -1,
            flags: 0,
        }
    }
}
impl TextDisplayState {
    #[inline]
    pub fn has_shadow(&self) -> bool {
        (self.flags & 1) != 0
    }
    #[inline]
    pub fn is_see_through(&self) -> bool {
        (self.flags & 2) != 0
    }
    #[inline]
    pub fn use_default_background(&self) -> bool {
        (self.flags & 4) != 0
    }
    #[inline]
    pub fn alignment(&self) -> TextDisplayAlignment {
        TextDisplayAlignment::from_flags(self.flags)
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct TextDisplayCachedLine {
    pub glyphs: Vec<WorldTextGlyph>,
    pub width: f32,
}
pub fn split_text_display_lines(
    font: &FontRenderer,
    raw_text: &str,
    line_width: i32,
) -> (Vec<TextDisplayCachedLine>, f32) {
    if raw_text.is_empty() {
        return (Vec::new(), 0.0);
    }
    let raw_glyphs = ansi_world_glyphs(raw_text);
    let max_wrap_width = if line_width > 0 {
        line_width as f32
    } else {
        f32::MAX
    };
    let mut lines: Vec<TextDisplayCachedLine> = Vec::new();
    let mut current_line: Vec<WorldTextGlyph> = Vec::new();
    let mut current_width: f32 = 0.0;
    let mut max_line_width: f32 = 0.0;
    let mut last_space_idx: Option<usize> = None;
    let mut width_at_last_space: f32 = 0.0;
    for glyph_item in raw_glyphs {
        let ch = glyph_item.ch;
        if ch == '\r' {
            continue;
        }
        if ch == '\n' {
            max_line_width = max_line_width.max(current_width);
            lines.push(TextDisplayCachedLine {
                glyphs: std::mem::take(&mut current_line),
                width: current_width,
            });
            current_width = 0.0;
            last_space_idx = None;
            width_at_last_space = 0.0;
            continue;
        }
        let g = font.glyph(ch);
        let advance = if glyph_item.bold {
            g.advance + 1.0
        } else {
            g.advance
        };
        if current_width + advance > max_wrap_width && !current_line.is_empty() {
            if let Some(space_pos) = last_space_idx {
                let next_line_glyphs = current_line.split_off(space_pos + 1);
                if current_line.last().is_some_and(|g| g.ch == ' ') {
                    current_line.pop();
                }
                max_line_width = max_line_width.max(width_at_last_space);
                lines.push(TextDisplayCachedLine {
                    glyphs: std::mem::take(&mut current_line),
                    width: width_at_last_space.max(0.0),
                });
                current_line = next_line_glyphs;
                let mut running_w = 0.0;
                last_space_idx = None;
                width_at_last_space = 0.0;
                for (idx, item) in current_line.iter().enumerate() {
                    if item.ch == ' ' {
                        last_space_idx = Some(idx);
                        width_at_last_space = running_w;
                    }
                    let g = font.glyph(item.ch);
                    let adv = if item.bold {
                        g.advance + 1.0
                    } else {
                        g.advance
                    };
                    running_w += adv;
                }
                current_width = running_w;
            } else {
                max_line_width = max_line_width.max(current_width);
                lines.push(TextDisplayCachedLine {
                    glyphs: std::mem::take(&mut current_line),
                    width: current_width,
                });
                current_width = 0.0;
                last_space_idx = None;
                width_at_last_space = 0.0;
            }
        }
        if ch == ' ' {
            last_space_idx = Some(current_line.len());
            width_at_last_space = current_width;
        }
        current_width += advance;
        current_line.push(glyph_item);
    }
    if raw_text.ends_with('\n') {
        max_line_width = max_line_width.max(current_width);
        lines.push(TextDisplayCachedLine {
            glyphs: current_line,
            width: current_width,
        });
        lines.push(TextDisplayCachedLine {
            glyphs: Vec::new(),
            width: 0.0,
        });
    } else if !current_line.is_empty() || lines.is_empty() {
        max_line_width = max_line_width.max(current_width);
        lines.push(TextDisplayCachedLine {
            glyphs: current_line,
            width: current_width,
        });
    }
    (lines, max_line_width.max(1.0))
}
#[inline]
pub fn unpack_argb_color(argb: i32, alpha_override: Option<f32>) -> [f32; 4] {
    let a_int = ((argb >> 24) & 0xFF) as u8;
    let r_int = ((argb >> 16) & 0xFF) as u8;
    let g_int = ((argb >> 8) & 0xFF) as u8;
    let b_int = (argb & 0xFF) as u8;
    let base_a = a_int as f32 / 255.0;
    let final_a = alpha_override.unwrap_or(base_a);
    [
        r_int as f32 / 255.0,
        g_int as f32 / 255.0,
        b_int as f32 / 255.0,
        final_a,
    ]
}
pub fn compute_text_display_matrix(
    entity_pos: Vec3,
    entity_yaw_deg: f32,
    entity_pitch_deg: f32,
    camera_yaw_deg: f32,
    camera_pitch_deg: f32,
    state: &TextDisplayState,
    max_line_width: f32,
    total_height: f32,
) -> Mat4 {
    let orientation = state.billboard.compute_orientation(
        entity_yaw_deg,
        entity_pitch_deg,
        camera_yaw_deg,
        camera_pitch_deg,
    );
    let m_pos = Mat4::from_translation(entity_pos);
    let m_billboard = Mat4::from_quat(orientation);
    let m_transform = state.transformation.to_mat4();
    let m_rot_y_pi = Mat4::from_rotation_y(std::f32::consts::PI);
    let m_scale = Mat4::from_scale(Vec3::splat(-0.025));
    let m_pivot = Mat4::from_translation(Vec3::new(1.0 - max_line_width * 0.5, -total_height, 0.0));
    m_pos * m_billboard * m_transform * m_rot_y_pi * m_scale * m_pivot
}
