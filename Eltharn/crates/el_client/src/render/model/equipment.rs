use crate::render::model::player::player_model::ArmorSlot;
use crate::resources::manager::ClientResourceManager;
use el_core::item::{DataComponent, ItemStack};
use el_core::resource_location::ResourceLocation;
use el_nbt::Tag;
use image::{DynamicImage, RgbaImage};
use serde::Deserialize;
use std::collections::HashMap;
#[derive(Clone, Debug)]
pub struct ArmorTextureLayer {
    pub texture_key: String,
    pub texture: ResourceLocation,
    pub color: [f32; 4],
    pub has_foil: bool,
    pub decal: bool,
    pub image: Option<DynamicImage>,
}
#[derive(Deserialize)]
struct EquipmentAsset {
    layers: HashMap<String, Vec<EquipmentLayer>>,
}
#[derive(Deserialize)]
struct EquipmentLayer {
    texture: String,
    #[serde(default)]
    dyeable: Option<Dyeable>,
    #[serde(default)]
    use_player_texture: bool,
}
#[derive(Deserialize)]
struct Dyeable {
    #[serde(default)]
    color_when_undyed: Option<i32>,
}
fn component_tag<'a>(stack: &'a ItemStack, name: &str) -> Option<&'a Tag> {
    match stack.components.get(name)? {
        DataComponent::Generic { value, .. } => Some(value),
        _ => None,
    }
}
fn field<'a>(tag: &'a Tag, key: &str) -> Option<&'a Tag> {
    match tag {
        Tag::Compound(fields) => fields.iter().find(|(name, _)| name == key).map(|(_, v)| v),
        _ => None,
    }
}
fn string(tag: &Tag) -> Option<&str> {
    match tag {
        Tag::String(value) => Some(value),
        _ => None,
    }
}
fn integer(tag: &Tag) -> Option<i32> {
    match tag {
        Tag::Int(value) => Some(*value),
        Tag::Byte(value) => Some(*value as i32),
        _ => None,
    }
}
fn slot_name(slot: ArmorSlot) -> &'static str {
    match slot {
        ArmorSlot::Head => "head",
        ArmorSlot::Chest => "chest",
        ArmorSlot::Legs => "legs",
        ArmorSlot::Feet => "feet",
    }
}
pub fn vanilla_armor_asset(item: &ResourceLocation) -> Option<(ArmorSlot, &'static str)> {
    if item.namespace != "minecraft" {
        return None;
    }
    if item.path == "turtle_helmet" {
        return Some((ArmorSlot::Head, "turtle_scute"));
    }
    let (material, piece) = item.path.rsplit_once('_')?;
    let asset = match material {
        "leather" => "leather",
        "copper" => "copper",
        "chainmail" => "chainmail",
        "iron" => "iron",
        "golden" => "gold",
        "diamond" => "diamond",
        "netherite" => "netherite",
        _ => return None,
    };
    let slot = match piece {
        "helmet" => ArmorSlot::Head,
        "chestplate" => ArmorSlot::Chest,
        "leggings" => ArmorSlot::Legs,
        "boots" => ArmorSlot::Feet,
        _ => return None,
    };
    Some((slot, asset))
}
pub fn armor_asset_for_slot(stack: &ItemStack, slot: ArmorSlot) -> Option<ResourceLocation> {
    if stack.is_empty() {
        return None;
    }
    let name = "minecraft:equippable";
    if let Some(component) = stack.components.get(name) {
        let DataComponent::Generic { value, .. } = component else {
            return None;
        };
        if string(field(value, "slot")?)? != slot_name(slot) {
            return None;
        }
        let asset = string(field(value, "asset_id")?)?;
        return Some(ResourceLocation::from_string(asset));
    }
    if stack.components.patch.map.contains_key(name) {
        return None;
    }
    let (expected_slot, asset) = vanilla_armor_asset(&stack.item.location)?;
    (expected_slot == slot).then(|| ResourceLocation::new("minecraft", asset))
}
pub fn equipment_has_layer(
    stack: &ItemStack,
    layer_type: &str,
    resources: &ClientResourceManager,
) -> bool {
    let asset = match stack.components.get("minecraft:equippable") {
        Some(DataComponent::Generic { value, .. }) => field(value, "asset_id")
            .and_then(string)
            .map(ResourceLocation::from_string),
        Some(_) => None,
        None if stack
            .components
            .patch
            .map
            .contains_key("minecraft:equippable") =>
        {
            None
        }
        None if stack.item.location.namespace == "minecraft"
            && stack.item.location.path == "elytra" =>
        {
            Some(ResourceLocation::new("minecraft", "elytra"))
        }
        None => vanilla_armor_asset(&stack.item.location)
            .map(|(_, id)| ResourceLocation::new("minecraft", id)),
    };
    let Some(asset) = asset else {
        return false;
    };
    let id = ResourceLocation::new(&asset.namespace, &format!("equipment/{}.json", asset.path));
    resources
        .read_resource_bytes(&id)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<EquipmentAsset>(&bytes).ok())
        .and_then(|asset| {
            asset
                .layers
                .get(layer_type)
                .map(|layers| !layers.is_empty())
        })
        .unwrap_or(false)
}
pub fn resolve_armor_layers(
    stack: &ItemStack,
    slot: ArmorSlot,
    resources: &ClientResourceManager,
) -> Vec<ArmorTextureLayer> {
    let Some(asset) = armor_asset_for_slot(stack, slot) else {
        return Vec::new();
    };
    let layer_type = if slot == ArmorSlot::Legs {
        "humanoid_leggings"
    } else {
        "humanoid"
    };
    let id = ResourceLocation::new(&asset.namespace, &format!("equipment/{}.json", asset.path));
    let Some(equipment) = resources
        .read_resource_bytes(&id)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<EquipmentAsset>(&bytes).ok())
    else {
        return Vec::new();
    };
    let Some(layers) = equipment.layers.get(layer_type) else {
        return Vec::new();
    };
    let dye = component_tag(stack, "minecraft:dyed_color")
        .and_then(|tag| integer(tag).or_else(|| field(tag, "rgb").and_then(integer)))
        .map(|rgb| rgb as u32 | 0xff00_0000);
    let mut foil = stack.has_foil();
    let mut result = Vec::with_capacity(layers.len() + 1);
    for layer in layers {
        let argb = match &layer.dyeable {
            Some(dyeable) => dye
                .or_else(|| {
                    dyeable
                        .color_when_undyed
                        .map(|rgb| rgb as u32 | 0xff00_0000)
                })
                .unwrap_or(0),
            None => 0xffff_ffff,
        };
        if argb == 0 {
            continue;
        }
        let _use_player_texture = layer.use_player_texture;
        let source = ResourceLocation::from_string(&layer.texture);
        let texture = ResourceLocation::new(
            &source.namespace,
            &format!("entity/equipment/{layer_type}/{}", source.path),
        );
        result.push(ArmorTextureLayer {
            texture_key: format!("armor-layer:{}:{}", texture.namespace, texture.path),
            texture,
            color: [
                ((argb >> 16) & 255) as f32 / 255.0,
                ((argb >> 8) & 255) as f32 / 255.0,
                (argb & 255) as f32 / 255.0,
                ((argb >> 24) & 255) as f32 / 255.0,
            ],
            has_foil: foil,
            decal: false,
            image: None,
        });
        foil = false;
    }
    if !result.is_empty()
        && let Some(trim) = resolve_trim(stack, &asset, layer_type, resources)
    {
        result.push(trim);
    }
    result
}
fn resolve_trim(
    stack: &ItemStack,
    asset: &ResourceLocation,
    layer_type: &str,
    resources: &ClientResourceManager,
) -> Option<ArmorTextureLayer> {
    let trim = component_tag(stack, "minecraft:trim")?;
    let material = field(trim, "material")?;
    let pattern = field(trim, "pattern")?;
    let base_suffix = field(material, "asset_name")
        .or_else(|| field(material, "assert_name"))
        .and_then(string)?;
    let overrides = field(material, "override_armor_assets");
    let suffix = overrides
        .and_then(|tag| match tag {
            Tag::Compound(values) => values
                .iter()
                .find(|(id, _)| id == &asset.to_string())
                .and_then(|(_, v)| string(v)),
            Tag::List(entries) => entries.iter().find_map(|entry| {
                let Tag::List(pair) = entry else { return None };
                (pair.len() == 2 && string(&pair[0]) == Some(asset.to_string().as_str()))
                    .then(|| string(&pair[1]))
                    .flatten()
            }),
            _ => None,
        })
        .unwrap_or(base_suffix);
    let pattern_id = ResourceLocation::from_string(string(field(pattern, "asset_id")?)?);
    let decal = field(pattern, "decal")
        .and_then(integer)
        .is_some_and(|value| value != 0);
    let texture = ResourceLocation::new(
        &pattern_id.namespace,
        &format!("trims/entity/{layer_type}/{}_{suffix}", pattern_id.path),
    );
    let image = palette_trim(
        resources,
        &ResourceLocation::new(
            &pattern_id.namespace,
            &format!("trims/entity/{layer_type}/{}", pattern_id.path),
        ),
        suffix,
    )?;
    Some(ArmorTextureLayer {
        texture_key: format!("armor-trim:{}:{}", texture.namespace, texture.path),
        texture,
        color: [1.0; 4],
        has_foil: false,
        decal,
        image: Some(DynamicImage::ImageRgba8(image)),
    })
}
fn palette_trim(
    resources: &ClientResourceManager,
    base: &ResourceLocation,
    suffix: &str,
) -> Option<RgbaImage> {
    let atlas_id = ResourceLocation::new("minecraft", "atlases/armor_trims.json");
    let atlas: serde_json::Value =
        serde_json::from_slice(&resources.read_resource_bytes(&atlas_id).ok()?).ok()?;
    let source = atlas.get("sources")?.as_array()?.iter().find(|source| {
        source.get("type").and_then(serde_json::Value::as_str)
            == Some("minecraft:paletted_permutations")
            && source
                .get("textures")
                .and_then(serde_json::Value::as_array)
                .is_some_and(|textures| {
                    textures
                        .iter()
                        .any(|v| v.as_str() == Some(base.to_string().as_str()))
                })
            && source
                .get("permutations")
                .and_then(|v| v.get(suffix))
                .is_some()
    })?;
    let key_id = ResourceLocation::from_string(source.get("palette_key")?.as_str()?);
    let value_id =
        ResourceLocation::from_string(source.get("permutations")?.get(suffix)?.as_str()?);
    let keys = resources.read_image(&key_id).ok()?.to_rgba8();
    let values = resources.read_image(&value_id).ok()?.to_rgba8();
    let mut image = resources.read_image(base).ok()?.to_rgba8();
    apply_palette(&mut image, &keys, &values)?;
    Some(image)
}
fn apply_palette(image: &mut RgbaImage, keys: &RgbaImage, values: &RgbaImage) -> Option<()> {
    if keys.pixels().len() != values.pixels().len() {
        return None;
    }
    let palette: HashMap<[u8; 3], image::Rgba<u8>> = keys
        .pixels()
        .zip(values.pixels())
        .filter(|(key, _)| key[3] != 0)
        .map(|(key, value)| ([key[0], key[1], key[2]], *value))
        .collect();
    for pixel in image.pixels_mut() {
        if pixel[3] == 0 {
            continue;
        }
        if let Some(value) = palette.get(&[pixel[0], pixel[1], pixel[2]]) {
            *pixel = image::Rgba([
                value[0],
                value[1],
                value[2],
                (u16::from(pixel[3]) * u16::from(value[3]) / 255) as u8,
            ]);
        }
    }
    Some(())
}
