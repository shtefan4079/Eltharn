use el_core::item::{DataComponent, ItemStack};
use el_nbt::Tag;
use serde::{Deserialize, Serialize};
pub mod banner;
pub mod bell;
pub mod book;
pub mod chest;
pub mod conduit;
pub mod copper_golem_statue;
pub mod decorated_pot;
pub mod end_cube;
pub mod player_head;
pub mod shield;
pub mod shulker_box;
pub mod skull;
pub mod trident;
pub use chest::ChestType;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpecialRenderLayer {
    pub texture: String,
    pub parts: Vec<SpecialModelPart>,
    pub tint: [f32; 4],
    pub crop_pixels: Option<[f32; 4]>,
}
impl SpecialRenderLayer {
    pub fn new(texture: impl Into<String>, parts: Vec<SpecialModelPart>, tint: [f32; 4]) -> Self {
        Self {
            texture: texture.into(),
            parts,
            tint,
            crop_pixels: None,
        }
    }
    pub fn with_crop_pixels(mut self, crop_pixels: Option<[f32; 4]>) -> Self {
        self.crop_pixels = crop_pixels;
        self
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpecialModelPart {
    pub pose: SpecialPartPose,
    pub cubes: Vec<SpecialModelCube>,
    pub children: Vec<SpecialModelPart>,
    pub color: [f32; 4],
}
impl SpecialModelPart {
    pub fn new(
        pose: SpecialPartPose,
        cubes: Vec<SpecialModelCube>,
        children: Vec<SpecialModelPart>,
    ) -> Self {
        Self {
            pose,
            cubes,
            children,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }
    pub fn new_colored(
        pose: SpecialPartPose,
        cubes: Vec<SpecialModelCube>,
        children: Vec<SpecialModelPart>,
        color: [f32; 4],
    ) -> Self {
        Self {
            pose,
            cubes,
            children,
            color,
        }
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpecialPartPose {
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}
impl SpecialPartPose {
    pub fn zero() -> Self {
        Self {
            offset: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        }
    }
    pub fn offset(x: f32, y: f32, z: f32) -> Self {
        Self {
            offset: [x, y, z],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        }
    }
    pub fn offset_and_rotation(x: f32, y: f32, z: f32, xr: f32, yr: f32, zr: f32) -> Self {
        Self {
            offset: [x, y, z],
            rotation: [xr, yr, zr],
            scale: [1.0; 3],
        }
    }
    pub fn scaled(mut self, s: f32) -> Self {
        self.scale = [self.scale[0] * s, self.scale[1] * s, self.scale[2] * s];
        self
    }
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub struct SpecialModelCube {
    pub tex_offs: [f32; 2],
    pub origin: [f32; 3],
    pub dimensions: [f32; 3],
    pub grow: [f32; 3],
    pub mirror: bool,
    pub tex_size: [f32; 2],
    pub visible_faces: [bool; 6],
}
impl SpecialModelCube {
    pub fn new(
        tex_offs: [f32; 2],
        origin: [f32; 3],
        dimensions: [f32; 3],
        tex_size: [f32; 2],
    ) -> Self {
        Self {
            tex_offs,
            origin,
            dimensions,
            grow: [0.0; 3],
            mirror: false,
            tex_size,
            visible_faces: [true; 6],
        }
    }
    pub fn grown(mut self, grow: f32) -> Self {
        self.grow = [grow; 3];
        self
    }
    pub fn mirrored(mut self) -> Self {
        self.mirror = true;
        self
    }
    pub fn visible_faces(mut self, faces: [bool; 6]) -> Self {
        self.visible_faces = faces;
        self
    }
}
pub struct PoseStack;
pub struct SubmitNodeCollector;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vector3fc {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}
pub struct EntityModelSet;
pub struct SpriteGetter;
pub struct PlayerSkinRenderCache;
pub trait BakingContext {
    fn entity_model_set(&self) -> &EntityModelSet;
    fn sprites(&self) -> &SpriteGetter;
    fn player_skin_render_cache(&self) -> &PlayerSkinRenderCache;
}
pub trait SpecialModelRenderer: Send + Sync {
    fn get_parts(&self, tint: [f32; 4]) -> Vec<SpecialModelPart>;
    fn get_texture(&self) -> String;
    fn get_layers(&self, _stack: Option<&ItemStack>, tint: [f32; 4]) -> Vec<SpecialRenderLayer> {
        let model_tint = self.get_tint();
        let layer_tint = multiply_color(tint, model_tint);
        vec![
            SpecialRenderLayer::new(self.get_texture(), self.get_parts(layer_tint), layer_tint)
                .with_crop_pixels(self.get_crop_pixels()),
        ]
    }
    fn get_crop_pixels(&self) -> Option<[f32; 4]> {
        None
    }
    fn get_tint(&self) -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0]
    }
}
pub(crate) fn multiply_color(left: [f32; 4], right: [f32; 4]) -> [f32; 4] {
    [
        left[0] * right[0],
        left[1] * right[1],
        left[2] * right[2],
        left[3] * right[3],
    ]
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BannerPatternLayer {
    pub pattern: String,
    pub color: String,
}
pub(crate) fn banner_pattern_layers(stack: Option<&ItemStack>) -> Vec<BannerPatternLayer> {
    let Some(stack) = stack else {
        return Vec::new();
    };
    let Some(component) = stack.components.get("minecraft:banner_patterns") else {
        return Vec::new();
    };
    let DataComponent::Generic { value, .. } = component else {
        return Vec::new();
    };
    let list = match value {
        Tag::List(items) => items.as_slice(),
        Tag::Compound(fields) => match compound_get(fields, "layers") {
            Some(Tag::List(items)) => items.as_slice(),
            _ => return Vec::new(),
        },
        _ => return Vec::new(),
    };
    list.iter()
        .filter_map(|tag| match tag {
            Tag::Compound(fields) => {
                let pattern = compound_string(fields, "pattern")
                    .or_else(|| compound_string(fields, "asset_id"))
                    .or_else(|| compound_string(fields, "id"))?;
                let color = dye_color_name_from_tag(compound_get(fields, "color")?);
                let color = color?;
                Some(BannerPatternLayer {
                    pattern: banner_pattern_texture_name(&pattern),
                    color: strip_minecraft_prefix(&color).to_string(),
                })
            }
            _ => None,
        })
        .collect()
}
pub(crate) fn stack_dye_color(stack: Option<&ItemStack>, key: &str) -> Option<String> {
    let component = stack?.components.get(key)?;
    match component {
        DataComponent::Generic {
            value: Tag::String(color),
            ..
        } => Some(strip_minecraft_prefix(color).to_string()),
        DataComponent::Generic {
            value: Tag::Compound(fields),
            ..
        } => compound_string(fields, "id")
            .or_else(|| compound_string(fields, "name"))
            .or_else(|| compound_string(fields, "color"))
            .map(|color| strip_minecraft_prefix(&color).to_string()),
        _ => None,
    }
}
pub(crate) fn pot_decoration_items(stack: Option<&ItemStack>) -> [Option<String>; 4] {
    let Some(stack) = stack else {
        return [None, None, None, None];
    };
    let Some(DataComponent::Generic { value, .. }) =
        stack.components.get("minecraft:pot_decorations")
    else {
        return [None, None, None, None];
    };
    let mut out: [Option<String>; 4] = [None, None, None, None];
    match value {
        Tag::List(items) => {
            for (idx, tag) in items.iter().take(4).enumerate() {
                out[idx] = tag_to_item_path(tag);
            }
        }
        Tag::Compound(fields) => {
            for (idx, key) in ["back", "left", "right", "front"].iter().enumerate() {
                out[idx] = compound_get(fields, key).and_then(tag_to_item_path);
            }
        }
        _ => {}
    }
    out
}
pub(crate) fn dye_color_rgb(color: &str) -> Option<[f32; 4]> {
    let rgb = match strip_minecraft_prefix(color) {
        "white" => 0xF9FFFE,
        "orange" => 0xF9801D,
        "magenta" => 0xC74EBD,
        "light_blue" => 0x3AB3DA,
        "yellow" => 0xFED83D,
        "lime" => 0x80C71F,
        "pink" => 0xF38BAA,
        "gray" => 0x474F52,
        "light_gray" => 0x9D9D97,
        "cyan" => 0x169C9C,
        "purple" => 0x8932B8,
        "blue" => 0x3C44AA,
        "brown" => 0x835432,
        "green" => 0x5E7C16,
        "red" => 0xB02E26,
        "black" => 0x1D1D21,
        _ => return None,
    };
    Some([
        ((rgb >> 16) & 0xFF) as f32 / 255.0,
        ((rgb >> 8) & 0xFF) as f32 / 255.0,
        (rgb & 0xFF) as f32 / 255.0,
        1.0,
    ])
}
pub(crate) fn strip_minecraft_prefix(value: &str) -> &str {
    value.strip_prefix("minecraft:").unwrap_or(value)
}
fn dye_color_name_from_tag(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(s) => Some(s.clone()),
        Tag::Int(id) => Some(dye_id_to_name(*id as u8).to_string()),
        Tag::Byte(id) => Some(dye_id_to_name(*id as u8).to_string()),
        _ => None,
    }
}
fn dye_id_to_name(id: u8) -> &'static str {
    match id {
        0 => "white",
        1 => "orange",
        2 => "magenta",
        3 => "light_blue",
        4 => "yellow",
        5 => "lime",
        6 => "pink",
        7 => "gray",
        8 => "light_gray",
        9 => "cyan",
        10 => "purple",
        11 => "blue",
        12 => "brown",
        13 => "green",
        14 => "red",
        15 => "black",
        _ => "white",
    }
}
pub(crate) fn compound_get<'a>(fields: &'a [(String, Tag)], key: &str) -> Option<&'a Tag> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
pub(crate) fn compound_string(fields: &[(String, Tag)], key: &str) -> Option<String> {
    match compound_get(fields, key)? {
        Tag::String(value) => Some(value.clone()),
        Tag::Compound(nested) => compound_string(nested, "id")
            .or_else(|| compound_string(nested, "asset_id"))
            .or_else(|| compound_string(nested, "name")),
        _ => None,
    }
}
fn tag_to_item_path(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(value) => Some(strip_minecraft_prefix(value).to_string()),
        Tag::Compound(fields) => compound_string(fields, "id")
            .or_else(|| compound_string(fields, "item"))
            .map(|item| strip_minecraft_prefix(&item).to_string()),
        _ => None,
    }
}
fn banner_pattern_texture_name(pattern: &str) -> String {
    let pattern = strip_minecraft_prefix(pattern);
    match pattern {
        "base" => "base",
        "square_bottom_left" | "bl" => "square_bottom_left",
        "square_bottom_right" | "br" => "square_bottom_right",
        "square_top_left" | "tl" => "square_top_left",
        "square_top_right" | "tr" => "square_top_right",
        "stripe_bottom" | "bs" => "stripe_bottom",
        "stripe_top" | "ts" => "stripe_top",
        "stripe_left" | "ls" => "stripe_left",
        "stripe_right" | "rs" => "stripe_right",
        "stripe_center" | "cs" => "stripe_center",
        "stripe_middle" | "ms" => "stripe_middle",
        "stripe_downright" | "drs" => "stripe_downright",
        "stripe_downleft" | "dls" => "stripe_downleft",
        "small_stripes" | "ss" => "small_stripes",
        "cross" | "cr" => "cross",
        "straight_cross" | "sc" => "straight_cross",
        "triangle_bottom" | "bt" => "triangle_bottom",
        "triangle_top" | "tt" => "triangle_top",
        "triangles_bottom" | "bts" => "triangles_bottom",
        "triangles_top" | "tts" => "triangles_top",
        "diagonal_left" | "ld" => "diagonal_left",
        "diagonal_up_right" | "rd" => "diagonal_up_right",
        "diagonal_up_left" | "lud" => "diagonal_up_left",
        "diagonal_right" | "rud" => "diagonal_right",
        "circle" | "mc" => "circle",
        "rhombus" | "mr" => "rhombus",
        "half_vertical" | "vh" => "half_vertical",
        "half_horizontal" | "hh" => "half_horizontal",
        "half_vertical_right" | "vhr" => "half_vertical_right",
        "half_horizontal_bottom" | "hhb" => "half_horizontal_bottom",
        "border" | "bo" => "border",
        "curly_border" | "cbo" => "curly_border",
        "gradient" | "gra" => "gradient",
        "gradient_up" | "gru" => "gradient_up",
        "bricks" | "bri" => "bricks",
        "globe" | "glb" => "globe",
        "creeper" | "cre" => "creeper",
        "skull" | "sku" => "skull",
        "flower" | "flo" => "flower",
        "mojang" | "moj" => "mojang",
        "piglin" | "pig" => "piglin",
        "flow" => "flow",
        "guster" => "guster",
        other => other,
    }
    .to_string()
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum UnbakedSpecialModel {
    #[serde(rename = "minecraft:shield", alias = "shield")]
    Shield,
    #[serde(rename = "minecraft:banner", alias = "banner")]
    Banner {
        color: String,
        #[serde(default = "default_attachment")]
        attachment: String,
    },
    #[serde(rename = "minecraft:chest", alias = "chest")]
    Chest {
        texture: String,
        #[serde(default)]
        openness: f32,
        #[serde(default)]
        chest_type: ChestType,
    },
    #[serde(rename = "minecraft:shulker_box", alias = "shulker_box")]
    ShulkerBox {
        texture: String,
        #[serde(default)]
        openness: f32,
    },
    #[serde(rename = "minecraft:decorated_pot", alias = "decorated_pot")]
    DecoratedPot,
    #[serde(rename = "minecraft:conduit", alias = "conduit")]
    Conduit,
    #[serde(rename = "minecraft:trident", alias = "trident")]
    Trident,
    #[serde(rename = "minecraft:head", alias = "head")]
    Head {
        kind: String,
        texture: Option<String>,
        #[serde(default)]
        animation: f32,
    },
    #[serde(rename = "minecraft:player_head", alias = "player_head")]
    PlayerHead,
    #[serde(
        rename = "minecraft:copper_golem_statue",
        alias = "copper_golem_statue"
    )]
    CopperGolemStatue { texture: String, pose: String },
    #[serde(rename = "minecraft:end_cube", alias = "end_cube")]
    EndCube { effect: String },
    #[serde(rename = "minecraft:bell", alias = "bell")]
    Bell,
    #[serde(rename = "minecraft:book", alias = "book")]
    Book {
        open_angle: f32,
        page1: f32,
        page2: f32,
    },
}
fn default_attachment() -> String {
    "ground".to_string()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SpecialItemModelJson {
    #[serde(rename = "type")]
    pub model_type: String,
    pub base: String,
    pub model: UnbakedSpecialModel,
}
impl UnbakedSpecialModel {
    pub fn bake(&self) -> Box<dyn SpecialModelRenderer> {
        match self {
            UnbakedSpecialModel::Shield => Box::new(shield::ShieldSpecialRenderer {}),
            UnbakedSpecialModel::Banner { color, attachment } => {
                Box::new(banner::BannerSpecialRenderer {
                    base_color: color.clone(),
                    attachment: attachment.clone(),
                })
            }
            UnbakedSpecialModel::Chest {
                texture,
                openness,
                chest_type,
            } => Box::new(chest::ChestSpecialRenderer {
                texture: texture.clone(),
                openness: *openness,
                chest_type: *chest_type,
            }),
            UnbakedSpecialModel::ShulkerBox { texture, openness } => {
                Box::new(shulker_box::ShulkerBoxSpecialRenderer {
                    texture: texture.clone(),
                    openness: *openness,
                })
            }
            UnbakedSpecialModel::DecoratedPot => {
                Box::new(decorated_pot::DecoratedPotSpecialRenderer {})
            }
            UnbakedSpecialModel::Conduit => Box::new(conduit::ConduitSpecialRenderer {}),
            UnbakedSpecialModel::Trident => Box::new(trident::TridentSpecialRenderer {}),
            UnbakedSpecialModel::Head {
                kind,
                texture,
                animation,
            } => Box::new(skull::SkullSpecialRenderer {
                kind: kind.clone(),
                texture: texture.clone(),
                animation: *animation,
            }),
            UnbakedSpecialModel::PlayerHead => Box::new(player_head::PlayerHeadSpecialRenderer {}),
            UnbakedSpecialModel::CopperGolemStatue { texture, pose } => {
                Box::new(copper_golem_statue::CopperGolemStatueSpecialRenderer {
                    texture: texture.clone(),
                    pose: pose.clone(),
                })
            }
            UnbakedSpecialModel::EndCube { effect } => Box::new(end_cube::EndCubeSpecialRenderer {
                effect: effect.clone(),
            }),
            UnbakedSpecialModel::Bell => Box::new(bell::BellSpecialRenderer {}),
            UnbakedSpecialModel::Book {
                open_angle,
                page1,
                page2,
            } => Box::new(book::BookSpecialRenderer {
                open_angle: *open_angle,
                page1: *page1,
                page2: *page2,
            }),
        }
    }
}
