use crate::FontWidthMeasurer;
use std::collections::BTreeMap;
#[derive(Clone)]
struct Glyph {
    ch: char,
    style: String,
    bold: bool,
}
fn glyphs(text: &str) -> Vec<Glyph> {
    let mut output = Vec::new();
    let mut active: BTreeMap<&str, String> = BTreeMap::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            let mut raw = String::new();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
                raw.push(next);
            }
            let codes = raw
                .split(';')
                .map(|value| value.parse::<u32>().unwrap_or(0))
                .collect::<Vec<_>>();
            let mut index = 0;
            while index < codes.len() {
                let code = codes[index];
                match code {
                    0 => active.clear(),
                    1 => {
                        active.insert("bold", "1".into());
                    }
                    3 => {
                        active.insert("italic", "3".into());
                    }
                    4 => {
                        active.insert("underline", "4".into());
                    }
                    8 => {
                        active.insert("obfuscated", "8".into());
                    }
                    9 => {
                        active.insert("strike", "9".into());
                    }
                    22 => {
                        active.remove("bold");
                    }
                    23 => {
                        active.remove("italic");
                    }
                    24 => {
                        active.remove("underline");
                    }
                    28 => {
                        active.remove("obfuscated");
                    }
                    29 => {
                        active.remove("strike");
                    }
                    39 => {
                        active.remove("color");
                    }
                    30..=37 | 90..=97 => {
                        active.insert("color", code.to_string());
                    }
                    38 => {
                        let count = match codes.get(index + 1) {
                            Some(2) => 4,
                            Some(5) => 2,
                            _ => 0,
                        };
                        if count > 0 && index + count < codes.len() {
                            let color = codes[index..=index + count]
                                .iter()
                                .map(u32::to_string)
                                .collect::<Vec<_>>()
                                .join(";");
                            active.insert("color", color);
                            index += count;
                        }
                    }
                    _ => {}
                }
                index += 1;
            }
            continue;
        }
        if ch == '§' {
            if let Some(code) = chars.next() {
                let code = code.to_ascii_lowercase();
                if let Some(color) = legacy_color(code) {
                    active.clear();
                    active.insert(
                        "color",
                        format!(
                            "38;2;{};{};{}",
                            color >> 16,
                            (color >> 8) & 255,
                            color & 255
                        ),
                    );
                } else {
                    match code {
                        'r' => active.clear(),
                        'l' => {
                            active.insert("bold", "1".into());
                        }
                        'o' => {
                            active.insert("italic", "3".into());
                        }
                        'n' => {
                            active.insert("underline", "4".into());
                        }
                        'm' => {
                            active.insert("strike", "9".into());
                        }
                        'k' => {
                            active.insert("obfuscated", "8".into());
                        }
                        _ => {}
                    }
                }
            }
            continue;
        }
        output.push(Glyph {
            ch,
            style: active.values().cloned().collect::<Vec<_>>().join(";"),
            bold: active.contains_key("bold"),
        });
    }
    output
}
fn legacy_color(code: char) -> Option<u32> {
    const COLORS: [u32; 16] = [
        0x000000, 0x0000AA, 0x00AA00, 0x00AAAA, 0xAA0000, 0xAA00AA, 0xFFAA00, 0xAAAAAA, 0x555555,
        0x5555FF, 0x55FF55, 0x55FFFF, 0xFF5555, 0xFF55FF, 0xFFFF55, 0xFFFFFF,
    ];
    code.to_digit(16).map(|ordinal| COLORS[ordinal as usize])
}
fn glyph_width(font: &dyn FontWidthMeasurer, glyph: &Glyph) -> f32 {
    font.measure_char(glyph.ch, 1.0) + if glyph.bold { 1.0 } else { 0.0 }
}
fn encode(glyphs: &[Glyph]) -> String {
    let mut output = String::new();
    let mut previous = "";
    for glyph in glyphs {
        if glyph.style != previous {
            output.push_str("\u{1b}[m");
            if !glyph.style.is_empty() {
                output.push_str(&format!("\u{1b}[{}m", glyph.style));
            }
            previous = &glyph.style;
        }
        output.push(glyph.ch);
    }
    if !previous.is_empty() {
        output.push_str("\u{1b}[m");
    }
    output
}
pub(crate) fn has_formatting(text: &str) -> bool {
    text.contains('\u{1b}') || text.contains('§')
}
pub(crate) fn plain(text: &str) -> String {
    glyphs(text).into_iter().map(|glyph| glyph.ch).collect()
}
pub(crate) fn width(text: &str, font: &dyn FontWidthMeasurer) -> f32 {
    if !has_formatting(text) {
        return font.measure_width(text, 1.0);
    }
    glyphs(text)
        .iter()
        .map(|glyph| glyph_width(font, glyph))
        .sum()
}
pub(crate) fn truncate(text: &str, width: f32, font: &dyn FontWidthMeasurer) -> String {
    let glyphs = glyphs(text);
    let mut used = 0.0;
    let end = glyphs
        .iter()
        .position(|glyph| {
            used += glyph_width(font, glyph);
            used > width
        })
        .unwrap_or(glyphs.len());
    encode(&glyphs[..end])
}
pub(crate) fn wrap(text: &str, max_width: f32, font: &dyn FontWidthMeasurer) -> Vec<String> {
    let glyphs = glyphs(text);
    let mut lines = Vec::new();
    for paragraph in glyphs.split(|glyph| glyph.ch == '\n') {
        if paragraph.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut start = 0;
        while start < paragraph.len() {
            let mut width = 0.0;
            let mut nonzero = false;
            let mut space = None;
            let mut end = paragraph.len();
            for (index, glyph) in paragraph.iter().enumerate().skip(start) {
                if glyph.ch == ' ' {
                    space = Some(index);
                }
                let advance = glyph_width(font, glyph);
                width += advance;
                if nonzero && width > max_width.max(1.0) {
                    end = space.unwrap_or(index);
                    break;
                }
                nonzero |= advance != 0.0;
            }
            lines.push(encode(&paragraph[start..end]));
            start = end + usize::from(paragraph.get(end).is_some_and(|glyph| glyph.ch == ' '));
        }
    }
    lines
}
