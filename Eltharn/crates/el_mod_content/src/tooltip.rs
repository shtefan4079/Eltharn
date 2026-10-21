use crate::ContentId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_ITEMS: usize = 54;
pub const MAX_WIDGETS: usize = 192;
pub const MAX_BYTES: usize = 24 * 1024;
pub const MAX_NODES: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Stack {
    pub id: String,
    pub count: i32,
    pub components: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub reference: u16,
    pub slot: u16,
    pub id: String,
    pub count: i32,
    pub component_key: String,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contents {
    pub component: String,
    pub size: u16,
    pub truncated: bool,
    #[serde(default)]
    pub selected_slot: Option<u16>,
    pub items: Vec<Item>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Text {
    pub text: String,
    pub color: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Widget {
    Panel {
        rect: [i32; 4],
        color: u32,
    },
    Text {
        position: [i32; 2],
        text: String,
        color: u32,
    },
    Item {
        reference: u16,
        position: [i32; 2],
        #[serde(default)]
        count: Option<i32>,
        #[serde(default)]
        quantity: Option<String>,
        #[serde(default = "enabled")]
        components: bool,
    },
    Image {
        rect: [i32; 4],
        texture: String,
        uv: [u16; 4],
        #[serde(default)]
        nine_slice: Option<NineSlice>,
        #[serde(default = "white")]
        color: u32,
    },
    Sprite {
        rect: [i32; 4],
        sprite: String,
        #[serde(default = "white")]
        color: u32,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NineSlice {
    pub size: [u16; 2],
    pub border: [u16; 4],
    pub stretch_inner: bool,
}
fn white() -> u32 {
    u32::MAX
}
fn enabled() -> bool {
    true
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Buttons {
    pub pressed: Vec<String>,
    pub complete: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Canvas {
    #[serde(default)]
    pub owner: String,
    pub width: u16,
    pub height: u16,
    #[serde(default)]
    pub align: Alignment,
    #[serde(default)]
    pub flow: Flow,
    pub widgets: Vec<Widget>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Flow {
    #[default]
    Inline,
    Overlay,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LayoutContext {
    pub version: u32,
    pub owner: String,
    pub viewport: [u16; 2],
    pub pointer: [i32; 2],
    pub tooltip: [i32; 4],
    pub component_origin: [i32; 2],
    pub canvas_size: [u16; 2],
    pub align: Alignment,
    pub placement: Option<Placement>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Placement {
    pub origin: [i32; 2],
    pub background: bool,
}
impl LayoutContext {
    pub fn validate(&self) -> Result<(), String> {
        let owner = self
            .owner
            .strip_suffix("/client")
            .ok_or("Invalid tooltip layout owner")?;
        owner
            .parse::<el_extension_api::ExtensionId>()
            .map_err(|_| "Invalid tooltip layout owner")?;
        let coordinate = |value: i32| (-32768..=32768).contains(&value);
        if self.version != 1
            || self.viewport.iter().any(|n| !(1..=16384).contains(n))
            || self.canvas_size.iter().any(|n| !(1..=512).contains(n))
            || self.pointer.iter().any(|n| !coordinate(*n))
            || self.component_origin.iter().any(|n| !coordinate(*n))
            || self.tooltip[..2].iter().any(|n| !coordinate(*n))
            || !(0..=16384).contains(&self.tooltip[2])
            || !(-2..=16384).contains(&self.tooltip[3])
            || self
                .placement
                .as_ref()
                .is_some_and(|placement| placement.origin.iter().any(|n| !coordinate(*n)))
        {
            return Err("Invalid tooltip layout bounds".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Context {
    pub version: u32,
    pub stack: Stack,
    pub contents: Option<Contents>,
    pub modifiers: [bool; 3],
    pub buttons: Buttons,
    pub lines: Vec<Text>,
    #[serde(default = "enabled")]
    pub native_images: bool,
    pub canvas: Option<Canvas>,
}
impl Context {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1
            || self.stack.id.parse::<ContentId>().is_err()
            || self.stack.count <= 0
            || self.stack.components.len() > 128
            || self
                .stack
                .components
                .iter()
                .any(|id| id.parse::<ContentId>().is_err())
            || self.lines.len() > 48
            || self.lines.iter().any(|line| line.text.len() > 2048)
            || self.buttons.pressed.len() > 64
            || (!self.buttons.complete && !self.buttons.pressed.is_empty())
            || self.buttons.pressed.iter().any(|key| {
                key.is_empty() || key.len() > 64 || !key.bytes().all(|b| b.is_ascii_alphanumeric())
            })
            || self.buttons.pressed.iter().collect::<BTreeSet<_>>().len()
                != self.buttons.pressed.len()
        {
            return Err("Invalid tooltip stack/text bounds".into());
        }
        let mut references = BTreeSet::new();
        if let Some(contents) = &self.contents {
            if contents.component.parse::<ContentId>().is_err()
                || contents.size > 256
                || contents.items.len() > MAX_ITEMS
                || contents
                    .selected_slot
                    .is_some_and(|slot| slot >= contents.size)
            {
                return Err("Invalid tooltip content bounds".into());
            }
            for item in &contents.items {
                if item.id.parse::<ContentId>().is_err()
                    || item.count <= 0
                    || item.slot >= contents.size
                    || !references.insert(item.reference)
                    || item.component_key.is_empty()
                    || item.component_key.len() > 32
                    || !item
                        .component_key
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric())
                {
                    return Err("Invalid tooltip content reference".into());
                }
            }
        }
        if let Some(canvas) = &self.canvas {
            if !(1..=512).contains(&canvas.width)
                || !(1..=512).contains(&canvas.height)
                || canvas.widgets.len() > MAX_WIDGETS
                || canvas.owner.len() > 160
            {
                return Err("Invalid tooltip canvas bounds".into());
            }
            let mut quads = 0u32;
            for widget in &canvas.widgets {
                let inside = |x: i32, y: i32, w: i32, h: i32| {
                    x >= 0
                        && y >= 0
                        && w > 0
                        && h > 0
                        && x <= canvas.width as i32
                        && y <= canvas.height as i32
                        && w <= canvas.width as i32
                        && h <= canvas.height as i32
                        && x + w <= canvas.width as i32
                        && y + h <= canvas.height as i32
                };
                match widget {
                    Widget::Panel {
                        rect: [x, y, w, h], ..
                    } if !inside(*x, *y, *w, *h) => {
                        return Err("Tooltip panel outside canvas".into());
                    }
                    Widget::Text {
                        position: [x, y],
                        text,
                        ..
                    } if !inside(*x, *y, 1, 9)
                        || text.len() > 128
                        || text.chars().any(char::is_control) =>
                    {
                        return Err("Invalid tooltip canvas text".into());
                    }
                    Widget::Item {
                        reference,
                        position: [x, y],
                        count,
                        quantity,
                        ..
                    } if !references.contains(reference)
                        || !inside(*x, *y, 16, 16)
                        || count.is_some_and(|n| !(1..=2_000_000).contains(&n))
                        || quantity.as_ref().is_some_and(|text| {
                            text.len() > 16 || text.chars().any(char::is_control)
                        }) =>
                    {
                        return Err("Invalid tooltip canvas item".into());
                    }
                    Widget::Image {
                        rect: [x, y, w, h],
                        texture,
                        uv,
                        nine_slice,
                        ..
                    } => {
                        let id: ContentId =
                            texture.parse().map_err(|_| "Invalid tooltip texture")?;
                        if !inside(*x, *y, *w, *h)
                            || id.path().ends_with(".png")
                            || uv.iter().any(|v| *v > 10000)
                            || uv[0] >= uv[2]
                            || uv[1] >= uv[3]
                        {
                            return Err("Invalid tooltip texture region".into());
                        }
                        if let Some(slice) = nine_slice {
                            let [sw, sh] = slice.size.map(u32::from);
                            let [left, top, right, bottom] = slice.border.map(u32::from);
                            if sw == 0
                                || sh == 0
                                || sw > 2048
                                || sh > 2048
                                || left + right >= sw
                                || top + bottom >= sh
                                || *uv != [0, 0, 10000, 10000]
                            {
                                return Err("Invalid tooltip nine-slice source".into());
                            }
                            quads += if slice.stretch_inner {
                                9
                            } else {
                                ((*w as u32).div_ceil(sw - left - right) + 2)
                                    * ((*h as u32).div_ceil(sh - top - bottom) + 2)
                            };
                        } else {
                            quads += 1;
                        }
                        if quads > 4096 {
                            return Err("Tooltip texture geometry budget".into());
                        }
                    }
                    Widget::Sprite {
                        rect: [x, y, w, h],
                        sprite,
                        ..
                    } => {
                        let id: ContentId =
                            sprite.parse().map_err(|_| "Invalid tooltip GUI sprite")?;
                        if id.namespace() != "minecraft"
                            || id.path().ends_with(".png")
                            || !inside(*x, *y, *w, *h)
                        {
                            return Err("Invalid tooltip GUI sprite region".into());
                        }
                        quads += 1;
                        if quads > 4096 {
                            return Err("Tooltip texture geometry budget".into());
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}
