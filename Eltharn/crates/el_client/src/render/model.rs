use crate::resources::manager::{ClientResourceManager, ResourceError};
use el_core::resource_location::ResourceLocation;
use glam::{Mat4, Vec3};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
pub mod animation;
pub mod entity_model;
pub mod equipment;
pub mod geom;
pub mod humanoid_model;
pub mod player;
fn deserialize_textures<'de, D>(
    deserializer: D,
) -> Result<Option<HashMap<String, String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawTextureEntry {
        String(String),
        Object {
            sprite: String,
            #[serde(default, rename = "force_translucent")]
            _force_translucent: Option<bool>,
        },
    }
    let raw_map = Option::<HashMap<String, RawTextureEntry>>::deserialize(deserializer)?;
    let Some(raw_map) = raw_map else {
        return Ok(None);
    };
    let mut map = HashMap::new();
    for (k, entry) in raw_map {
        let val = match entry {
            RawTextureEntry::String(s) => s,
            RawTextureEntry::Object { sprite, .. } => sprite,
        };
        map.insert(k, val);
    }
    Ok(Some(map))
}
#[derive(Debug, Deserialize, Clone)]
pub struct BlockModel {
    pub parent: Option<String>,
    #[serde(deserialize_with = "deserialize_textures", default)]
    pub textures: Option<HashMap<String, String>>,
    pub elements: Option<Vec<ModelElement>>,
    pub display: Option<HashMap<String, ItemTransform>>,
    pub ambientocclusion: Option<bool>,
    #[serde(skip)]
    pub force_translucent_textures: HashSet<String>,
}
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
pub struct ItemTransform {
    #[serde(default)]
    pub rotation: [f32; 3],
    #[serde(default)]
    pub translation: [f32; 3],
    #[serde(default = "default_item_transform_scale")]
    pub scale: [f32; 3],
}
fn default_item_transform_scale() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
impl Default for ItemTransform {
    fn default() -> Self {
        Self {
            rotation: [0.0, 0.0, 0.0],
            translation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}
impl ItemTransform {
    pub fn matrix(self, apply_left_hand_fix: bool) -> Mat4 {
        let mut translation = Vec3::from(self.translation) * 0.0625;
        translation = translation.clamp(Vec3::splat(-5.0), Vec3::splat(5.0));
        let scale = Vec3::from(self.scale).clamp(Vec3::splat(-4.0), Vec3::splat(4.0));
        let rot_x = self.rotation[0].to_radians();
        let rot_y = if apply_left_hand_fix {
            -self.rotation[1]
        } else {
            self.rotation[1]
        }
        .to_radians();
        let rot_z = if apply_left_hand_fix {
            -self.rotation[2]
        } else {
            self.rotation[2]
        }
        .to_radians();
        if apply_left_hand_fix {
            translation.x = -translation.x;
        }
        Mat4::from_translation(translation)
            * Mat4::from_euler(glam::EulerRot::XYZ, rot_x, rot_y, rot_z)
            * Mat4::from_scale(scale)
            * Mat4::from_translation(Vec3::splat(-0.5))
    }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "RawElementRotation")]
pub struct ElementRotation {
    pub origin: [f32; 3],
    pub axis: String,
    pub angle: f32,
    pub euler: Option<[f32; 3]>,
    pub rescale: Option<bool>,
}
#[derive(Deserialize)]
struct RawElementRotation {
    origin: [f32; 3],
    #[serde(default)]
    axis: Option<String>,
    #[serde(default)]
    angle: Option<f32>,
    #[serde(default)]
    x: Option<f32>,
    #[serde(default)]
    y: Option<f32>,
    #[serde(default)]
    z: Option<f32>,
    #[serde(default)]
    rescale: Option<bool>,
}
impl TryFrom<RawElementRotation> for ElementRotation {
    type Error = String;
    fn try_from(raw: RawElementRotation) -> Result<Self, Self::Error> {
        let (axis, angle, euler) = match (raw.axis, raw.angle) {
            (None, None) => {
                if raw.x.is_none() && raw.y.is_none() && raw.z.is_none() {
                    return Err(
                        "missing rotation value, expected either 'axis' and 'angle' or 'x', 'y' and 'z'"
                            .to_string(),
                    );
                }
                (
                    String::new(),
                    0.0,
                    Some([
                        raw.x.unwrap_or(0.0),
                        raw.y.unwrap_or(0.0),
                        raw.z.unwrap_or(0.0),
                    ]),
                )
            }
            (axis, angle) => {
                let Some(axis) = axis else {
                    return Err("missing 'axis' for element rotation".to_string());
                };
                let Some(angle) = angle else {
                    return Err("missing 'angle' for element rotation".to_string());
                };
                let normalized = axis.to_lowercase();
                if !matches!(normalized.as_str(), "x" | "y" | "z") {
                    return Err(format!("invalid rotation axis: {axis}"));
                }
                (normalized, angle, None)
            }
        };
        Ok(Self {
            origin: raw.origin,
            axis,
            angle,
            euler,
            rescale: raw.rescale,
        })
    }
}
impl ElementRotation {
    pub fn matrix(&self) -> Mat4 {
        let mut transform = if let Some([x, y, z]) = self.euler {
            Mat4::from_rotation_z(z.to_radians())
                * Mat4::from_rotation_y(y.to_radians())
                * Mat4::from_rotation_x(x.to_radians())
        } else if self.angle == 0.0 {
            Mat4::IDENTITY
        } else {
            match self.axis.as_str() {
                "x" => Mat4::from_rotation_x(self.angle.to_radians()),
                "y" => Mat4::from_rotation_y(self.angle.to_radians()),
                "z" => Mat4::from_rotation_z(self.angle.to_radians()),
                _ => Mat4::IDENTITY,
            }
        };
        if self.rescale.unwrap_or(false) && transform != Mat4::IDENTITY {
            let scale_factor = |axis: Vec3| {
                let transformed = transform.transform_vector3(axis).abs();
                1.0 / transformed.max_element()
            };
            transform *= Mat4::from_scale(Vec3::new(
                scale_factor(Vec3::X),
                scale_factor(Vec3::Y),
                scale_factor(Vec3::Z),
            ));
        }
        transform
    }
}
#[derive(Debug, Deserialize, Clone)]
pub struct ModelElement {
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub rotation: Option<ElementRotation>,
    #[serde(default)]
    pub shade: Option<bool>,
    #[serde(default)]
    pub light_emission: u8,
    pub faces: HashMap<String, ModelFace>,
}
#[derive(Debug, Deserialize, Clone)]
pub struct ModelFace {
    pub texture: String,
    pub uv: Option<[f32; 4]>,
    pub cullface: Option<String>,
    pub rotation: Option<u32>,
    pub tintindex: Option<i32>,
}
impl BlockModel {
    pub fn load(
        resources: &ClientResourceManager,
        id: &ResourceLocation,
    ) -> Result<Self, ResourceError> {
        Self::load_with_ancestors(resources, id, &mut Vec::new())
    }
    fn load_with_ancestors(
        resources: &ClientResourceManager,
        id: &ResourceLocation,
        ancestors: &mut Vec<ResourceLocation>,
    ) -> Result<Self, ResourceError> {
        let mut model = Self::load_body(resources, id)?;
        ancestors.push(id.clone());
        if let Some(parent_str) = model.parent.clone() {
            if !parent_str.starts_with("builtin/") {
                let parent_id = ResourceLocation::from_string(&parent_str);
                let parent_path = if parent_id.path.contains('/') {
                    parent_id.path.clone()
                } else {
                    format!("block/{}", parent_id.path)
                };
                let parent_actual_id = ResourceLocation::new(&parent_id.namespace, &parent_path);
                if ancestors.contains(&parent_actual_id) {
                    warn_model_error_once(
                        id,
                        &format!("Model {id} ignored due to cyclic dependency"),
                    );
                } else {
                    match Self::load_with_ancestors(resources, &parent_actual_id, ancestors) {
                        Ok(parent_model) => {
                            if model.ambientocclusion.is_none() {
                                model.ambientocclusion = parent_model.ambientocclusion;
                            }
                            model
                                .force_translucent_textures
                                .extend(parent_model.force_translucent_textures.iter().cloned());
                            if let Some(parent_tex) = parent_model.textures {
                                let mut merged = parent_tex;
                                if let Some(self_tex) = model.textures.take() {
                                    merged.extend(self_tex);
                                }
                                model.textures = Some(merged);
                            }
                            if let Some(parent_display) = parent_model.display {
                                let mut merged = parent_display;
                                if let Some(self_display) = model.display.take() {
                                    merged.extend(self_display);
                                }
                                model.display = Some(merged);
                            }
                            if model.elements.is_none() {
                                model.elements = parent_model.elements;
                            }
                        }
                        Err(error) => {
                            warn_model_error_once(
                                id,
                                &format!(
                                    "Model {id}: failed to load parent {parent_actual_id}: {error}"
                                ),
                            );
                        }
                    }
                }
            }
        }
        ancestors.pop();
        Ok(model)
    }
    fn load_body(
        resources: &ClientResourceManager,
        id: &ResourceLocation,
    ) -> Result<Self, ResourceError> {
        let text = resources.read_model(id)?;
        let mut model: BlockModel =
            serde_json::from_str(&text).map_err(|e| ResourceError::Parse {
                id: id.clone(),
                message: e.to_string(),
            })?;
        if let Ok(raw) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(textures) = raw.get("textures").and_then(serde_json::Value::as_object) {
                for (name, value) in textures {
                    if value
                        .get("force_translucent")
                        .and_then(serde_json::Value::as_bool)
                        == Some(true)
                    {
                        model.force_translucent_textures.insert(name.clone());
                    }
                }
            }
        }
        Ok(model)
    }
    pub fn resolve_texture<'a>(&'a self, mut tex_name: &'a str) -> &'a str {
        let max_depth = 10;
        let mut depth = 0;
        while depth < max_depth {
            let key = if tex_name.starts_with('#') {
                &tex_name[1..]
            } else {
                tex_name
            };
            if let Some(textures) = &self.textures {
                if let Some(resolved) = textures.get(key) {
                    tex_name = resolved;
                    depth += 1;
                    continue;
                }
            }
            break;
        }
        tex_name
    }
    pub fn texture_forces_translucency<'a>(&'a self, mut tex_name: &'a str) -> bool {
        for _ in 0..10 {
            let key = tex_name.strip_prefix('#').unwrap_or(tex_name);
            if self.force_translucent_textures.contains(key) {
                return true;
            }
            let Some(next) = self
                .textures
                .as_ref()
                .and_then(|textures| textures.get(key))
            else {
                return false;
            };
            tex_name = next;
        }
        false
    }
    pub fn item_transform(&self, display_context: &str) -> ItemTransform {
        if let Some(display) = self.display.as_ref() {
            if let Some(t) = display.get(display_context) {
                return *t;
            }
            if display_context == "thirdperson_lefthand" {
                if let Some(t) = display.get("thirdperson_righthand") {
                    return *t;
                }
            } else if display_context == "firstperson_lefthand" {
                if let Some(t) = display.get("firstperson_righthand") {
                    return *t;
                }
            }
        }
        ItemTransform::default()
    }
}
fn warn_model_error_once(id: &ResourceLocation, message: &str) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let warned = WARNED.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut warned) = warned.lock() {
        if warned.insert(id.to_string()) {
            log::warn!("{message}");
        }
    }
}
