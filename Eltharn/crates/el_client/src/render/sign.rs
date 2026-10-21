use crate::ui::chat::chat_component::WorldTextGlyph;
use el_core::block_state::BlockState;
use el_nbt::Tag;
use glam::{Mat4, Vec3};
pub(crate) const SIGN_LINES: usize = 4;
pub(crate) const SIGN_TEXT_LINE_HEIGHT: f32 = 10.0;
pub(crate) const SIGN_MAX_TEXT_LINE_WIDTH: f32 = 90.0;
const SIGN_ROW_MIDPOINT: f32 = 2.0 * SIGN_TEXT_LINE_HEIGHT;
const STANDING_TEXT_OFFSET: Vec3 = Vec3::new(0.0, 0.333_333_34, 0.046_666_667);
const STANDING_TEXT_SCALE: f32 = 0.010_416_667;
const HANGING_TEXT_OFFSET: Vec3 = Vec3::new(0.0, -0.32, 0.073);
const HANGING_TEXT_SCALE: f32 = 0.014_062_5;
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct SignSideText {
    pub lines: [String; SIGN_LINES],
    pub color: String,
    pub has_glowing_text: bool,
}
impl Default for SignSideText {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            color: "black".to_string(),
            has_glowing_text: false,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct ParsedSignText {
    pub front: SignSideText,
    pub back: SignSideText,
}
fn dye_text_color(name: &str) -> Option<u32> {
    Some(match name {
        "white" => 15790320,
        "orange" => 15435844,
        "magenta" => 12801229,
        "light_blue" => 6719955,
        "yellow" => 14602026,
        "lime" => 4312372,
        "pink" => 14188952,
        "gray" => 4408131,
        "light_gray" => 11250603,
        "cyan" => 2651799,
        "purple" => 8073150,
        "blue" => 3949738,
        "brown" => 5320730,
        "green" => 3887386,
        "red" => 11743532,
        "black" => 1973019,
        _ => return None,
    })
}
fn scale_rgb(color: u32, factor: f32) -> u32 {
    let channel = |shift: u32| (((color >> shift) & 0xFF) as f32 * factor) as u32;
    (channel(16) << 16) | (channel(8) << 8) | channel(0)
}
pub(crate) fn dark_text_color(side: &SignSideText) -> Option<u32> {
    let color = dye_text_color(&side.color)?;
    if color == dye_text_color("black")? && side.has_glowing_text {
        Some((-988212_i32) as u32)
    } else {
        Some(scale_rgb(color, 0.4))
    }
}
fn component_nbt_to_json(tag: &Tag) -> Option<serde_json::Value> {
    use serde_json::{Map, Number, Value};
    Some(match tag {
        Tag::String(value) => Value::String(value.clone()),
        Tag::Byte(value) => Value::Bool(*value != 0),
        Tag::Short(value) => Value::Number(Number::from(*value)),
        Tag::Int(value) => Value::Number(Number::from(*value)),
        Tag::Long(value) => Value::Number(Number::from(*value)),
        Tag::Float(value) if value.is_finite() => {
            Value::Number(Number::from_f64(f64::from(*value))?)
        }
        Tag::Double(value) if value.is_finite() => Value::Number(Number::from_f64(*value)?),
        Tag::List(values) => Value::Array(
            values
                .iter()
                .map(component_nbt_to_json)
                .collect::<Option<Vec<_>>>()?,
        ),
        Tag::Compound(fields) => {
            let mut object = Map::new();
            for (key, value) in fields {
                object.insert(key.clone(), component_nbt_to_json(value)?);
            }
            Value::Object(object)
        }
        Tag::ByteArray(values) => Value::Array(
            values
                .iter()
                .map(|value| Value::Number(Number::from(*value)))
                .collect(),
        ),
        Tag::IntArray(values) => Value::Array(
            values
                .iter()
                .map(|value| Value::Number(Number::from(*value)))
                .collect(),
        ),
        Tag::LongArray(values) => Value::Array(
            values
                .iter()
                .map(|value| Value::Number(Number::from(*value)))
                .collect(),
        ),
        Tag::End | Tag::Float(_) | Tag::Double(_) => return None,
    })
}
fn parse_message_tag(tag: Option<&Tag>) -> String {
    match tag {
        None => String::new(),
        Some(Tag::String(value)) if value.is_empty() => String::new(),
        Some(Tag::String(value)) => match serde_json::from_str::<serde_json::Value>(value) {
            Ok(component @ (serde_json::Value::Object(_) | serde_json::Value::Array(_))) => {
                let mut out = String::new();
                crate::ui::chat::chat_component::json_component_to_ansi(&component, &mut out);
                out
            }
            _ => value.clone(),
        },
        Some(tag @ (Tag::Compound(_) | Tag::List(_))) => {
            let Some(component) = component_nbt_to_json(tag) else {
                return String::new();
            };
            let mut out = String::new();
            crate::ui::chat::chat_component::json_component_to_ansi(&component, &mut out);
            out
        }
        Some(_) => String::new(),
    }
}
fn parse_side_text(tag: Option<&Tag>) -> Option<SignSideText> {
    let Tag::Compound(fields) = tag? else {
        return None;
    };
    let mut side = SignSideText::default();
    if let Some(Tag::List(messages)) = el_nbt::compound_get(fields, "messages") {
        for (index, message) in messages.iter().take(SIGN_LINES).enumerate() {
            side.lines[index] = parse_message_tag(Some(message));
        }
    }
    if let Some(Tag::String(color)) = el_nbt::compound_get(fields, "color") {
        side.color = color.clone();
    }
    if let Some(Tag::Byte(glowing)) = el_nbt::compound_get(fields, "has_glowing_text") {
        side.has_glowing_text = *glowing != 0;
    }
    Some(side)
}
pub(crate) fn parse_sign_nbt(nbt: &[u8]) -> Option<ParsedSignText> {
    let Ok(Tag::Compound(root)) = el_nbt::read_any_compound(nbt) else {
        return None;
    };
    Some(ParsedSignText {
        front: parse_side_text(el_nbt::compound_get(&root, "front_text")).unwrap_or_default(),
        back: parse_side_text(el_nbt::compound_get(&root, "back_text")).unwrap_or_default(),
    })
}
pub(crate) fn is_sign_state(state: &BlockState) -> bool {
    state.block.namespace == "minecraft" && state.block.path.ends_with("_sign")
}
enum SignFamily {
    Standing { rotation_degrees: f32 },
    Wall { yaw_degrees: f32 },
    CeilingHanging { rotation_degrees: f32 },
    WallHanging { yaw_degrees: f32 },
}
fn facing_yaw(facing: Option<&String>) -> f32 {
    match facing.map(String::as_str) {
        Some("south") => 0.0,
        Some("west") => 90.0,
        Some("north") => 180.0,
        Some("east") => 270.0,
        _ => 0.0,
    }
}
fn rotation_segment_degrees(rotation: Option<&String>) -> f32 {
    rotation
        .and_then(|value| value.parse::<u8>().ok())
        .map(|segment| f32::from(segment % 16) * 360.0 / 16.0)
        .unwrap_or(0.0)
}
fn sign_family(state: &BlockState) -> SignFamily {
    let path = state.block.path.as_str();
    let facing = state.get_property("facing");
    if path.ends_with("_wall_hanging_sign") {
        SignFamily::WallHanging {
            yaw_degrees: facing_yaw(facing),
        }
    } else if path.ends_with("_hanging_sign") {
        SignFamily::CeilingHanging {
            rotation_degrees: rotation_segment_degrees(state.get_property("rotation")),
        }
    } else if path.ends_with("_wall_sign") {
        SignFamily::Wall {
            yaw_degrees: facing_yaw(facing),
        }
    } else {
        SignFamily::Standing {
            rotation_degrees: rotation_segment_degrees(state.get_property("rotation")),
        }
    }
}
fn yaw_rotation(yaw_degrees: f32) -> Mat4 {
    Mat4::from_rotation_y((-yaw_degrees).to_radians())
}
pub(crate) fn sign_face_matrix(state: &BlockState, front: bool) -> Mat4 {
    let scale = |s: f32| Mat4::from_scale(Vec3::new(s, -s, s));
    let back_flip = (!front)
        .then(|| Mat4::from_rotation_y(std::f32::consts::PI))
        .unwrap_or_default();
    match sign_family(state) {
        SignFamily::Standing { rotation_degrees } => {
            Mat4::from_translation(Vec3::new(0.5, 0.5, 0.5))
                * yaw_rotation(rotation_degrees)
                * back_flip
                * Mat4::from_translation(STANDING_TEXT_OFFSET)
                * scale(STANDING_TEXT_SCALE)
        }
        SignFamily::Wall { yaw_degrees } => {
            Mat4::from_translation(Vec3::new(0.5, 0.5, 0.5))
                * yaw_rotation(yaw_degrees)
                * Mat4::from_translation(Vec3::new(0.0, -0.3125, -0.4375))
                * back_flip
                * Mat4::from_translation(STANDING_TEXT_OFFSET)
                * scale(STANDING_TEXT_SCALE)
        }
        SignFamily::CeilingHanging { rotation_degrees } => {
            Mat4::from_translation(Vec3::new(0.5, 0.9375, 0.5))
                * yaw_rotation(rotation_degrees)
                * Mat4::from_translation(Vec3::new(0.0, -0.3125, 0.0))
                * back_flip
                * Mat4::from_translation(HANGING_TEXT_OFFSET)
                * scale(HANGING_TEXT_SCALE)
        }
        SignFamily::WallHanging { yaw_degrees } => {
            Mat4::from_translation(Vec3::new(0.5, 0.9375, 0.5))
                * yaw_rotation(yaw_degrees)
                * Mat4::from_translation(Vec3::new(0.0, -0.3125, 0.0))
                * back_flip
                * Mat4::from_translation(HANGING_TEXT_OFFSET)
                * scale(HANGING_TEXT_SCALE)
        }
    }
}
pub(crate) struct SignRowGlyph {
    pub glyph: WorldTextGlyph,
    pub advance: f32,
    pub rgb_override: Option<u32>,
}
pub(crate) struct SignRow {
    pub y: f32,
    pub tint_rgb: u32,
    pub glyphs: Vec<SignRowGlyph>,
}
pub(crate) fn sign_row_layout(
    parsed: &ParsedSignText,
    advance_of: impl Fn(char, bool) -> f32 + Copy,
) -> (Vec<SignRow>, Vec<SignRow>) {
    fn layout_side(
        side: &SignSideText,
        advance_of: impl Fn(char, bool) -> f32 + Copy,
    ) -> Vec<SignRow> {
        let Some(base_color) = dye_text_color(&side.color) else {
            return Vec::new();
        };
        let tint = if side.has_glowing_text {
            base_color
        } else {
            dark_text_color(side).unwrap_or(base_color)
        };
        let mut rows = Vec::new();
        for (index, line) in side.lines.iter().enumerate() {
            let glyphs = crate::ui::chat::chat_component::ansi_world_glyphs(line);
            if glyphs.is_empty() {
                continue;
            }
            let advances: Vec<f32> = glyphs
                .iter()
                .map(|glyph| advance_of(glyph.ch, glyph.bold))
                .collect();
            let total: f32 = advances.iter().sum();
            let cut = if total <= SIGN_MAX_TEXT_LINE_WIDTH {
                glyphs.len()
            } else {
                first_wrapped_line_cut(&glyphs, &advances)
            };
            let mut laid = Vec::with_capacity(cut);
            for (glyph, advance) in glyphs.into_iter().zip(advances).take(cut) {
                let rgb_override = (glyph.color != 0xFFFFFF).then_some(glyph.color);
                laid.push(SignRowGlyph {
                    glyph,
                    advance,
                    rgb_override,
                });
            }
            rows.push(SignRow {
                y: index as f32 * SIGN_TEXT_LINE_HEIGHT - SIGN_ROW_MIDPOINT,
                tint_rgb: tint,
                glyphs: laid,
            });
        }
        rows
    }
    (
        layout_side(&parsed.front, advance_of),
        layout_side(&parsed.back, advance_of),
    )
}
fn first_wrapped_line_cut(glyphs: &[WorldTextGlyph], advances: &[f32]) -> usize {
    let mut best_cut = 0usize;
    let mut x = 0.0;
    for (index, glyph) in glyphs.iter().enumerate() {
        if glyph.ch == ' ' && x <= SIGN_MAX_TEXT_LINE_WIDTH {
            best_cut = index.max(1);
        }
        x += advances[index];
        if x > SIGN_MAX_TEXT_LINE_WIDTH {
            break;
        }
    }
    if best_cut > 0 {
        return best_cut;
    }
    let mut x = 0.0;
    for (index, advance) in advances.iter().enumerate() {
        x += advance;
        if x > SIGN_MAX_TEXT_LINE_WIDTH {
            return index.max(1);
        }
    }
    glyphs.len()
}
