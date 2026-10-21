use crate::resources::manager::{ClientResourceManager, ResourceError};
use el_core::resource_location::ResourceLocation;
use el_extension_manager::content_assets::OwnedAssets;
use el_mod_content::{ContentId, Definition};
use serde::{Deserialize, Serialize};
const MODEL_JSON_LIMIT: usize = 262_144;
const MATERIAL_JSON_LIMIT: usize = 64 * 1024;
const CLIP_LIMIT: usize = 8 * 1024 * 1024;
const MAX_CLIPS: usize = 128;
fn default_color() -> [u8; 4] {
    [255, 255, 255, 255]
}
fn default_light() -> u8 {
    15
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Material {
    #[serde(default = "default_color")]
    pub color: [u8; 4],
    #[serde(default = "default_light")]
    pub light: u8,
    #[serde(default)]
    pub layer: Option<MaterialLayer>,
}
impl Default for Material {
    fn default() -> Self {
        Self {
            color: default_color(),
            light: default_light(),
            layer: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MaterialLayer {
    Solid,
    Cutout,
    Translucent,
}
fn check_light(light: u8) -> Result<(), String> {
    if light > 15 {
        return Err(format!("material light out of bounds 0..15: {light}"));
    }
    Ok(())
}
fn strict_json(bytes: &[u8], limit: usize) -> Result<serde_json::Value, String> {
    if bytes.len() > limit {
        return Err(format!("json exceeds {limit} bytes"));
    }
    el_mod_runtime::wire::strict_json(bytes, limit)
}
fn parse_content_id(s: &str) -> Result<ContentId, String> {
    s.parse::<ContentId>()
        .map_err(|e| format!("invalid content id '{s}': {e}"))
}
fn parse_material(bytes: &[u8]) -> Result<Material, String> {
    if bytes.len() > MATERIAL_JSON_LIMIT {
        return Err(format!("material exceeds {MATERIAL_JSON_LIMIT} bytes"));
    }
    let def = Definition::parse(bytes).map_err(|e| format!("material parse: {e}"))?;
    let mat: Material =
        serde_json::from_value(def.value()).map_err(|e| format!("bad material: {e}"))?;
    check_light(mat.light)?;
    Ok(mat)
}
pub fn load(resources: &ClientResourceManager, model: &ContentId) -> Result<Material, String> {
    let loc = ResourceLocation::from_string(model.as_str());
    let text = resources
        .read_model(&loc)
        .map_err(|e| format!("read model: {e:?}"))?;
    let text: &str = text.as_ref();
    let v = strict_json(text.as_bytes(), MODEL_JSON_LIMIT)?;
    let raw = match v.get("eltharn_material") {
        None => return Ok(Material::default()),
        Some(raw) => raw,
    };
    let s = raw
        .as_str()
        .ok_or_else(|| "eltharn_material must be string".to_string())?;
    let mat_id = parse_content_id(s)?;
    let mat_loc = ResourceLocation::from_string(&format!(
        "{}:materials/{}.json",
        mat_id.namespace(),
        mat_id.path()
    ));
    let bytes = resources
        .read_resource_bytes(&mat_loc)
        .map_err(|e| format!("read material: {e:?}"))?;
    parse_material(&bytes)
}
pub(super) fn is_not_found(err: &ResourceError) -> bool {
    match err {
        ResourceError::Io { source, .. } => source.kind() == std::io::ErrorKind::NotFound,
        ResourceError::PreparedSnapshot {
            operation: "read prepared base resource",
            ..
        } => true,
        _ => false,
    }
}
pub fn validate_audio_bytes(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() > CLIP_LIMIT {
        return Err("clip exceeds 8MiB".to_string());
    }
    if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
        return Ok(());
    }
    if bytes.len() >= 5 && &bytes[0..4] == b"OggS" && bytes[4] == 0 {
        return Ok(());
    }
    Err("unsupported clip container (want WAV RIFF..WAVE or OggS v0)".to_string())
}
pub fn sound_file(
    resources: &ClientResourceManager,
    sound: &ContentId,
) -> Result<ResourceLocation, String> {
    let ogg = ResourceLocation::from_string(&format!(
        "{}:sounds/{}.ogg",
        sound.namespace(),
        sound.path()
    ));
    let wav = ResourceLocation::from_string(&format!(
        "{}:sounds/{}.wav",
        sound.namespace(),
        sound.path()
    ));
    let o = resources.read_resource_bytes(&ogg);
    let w = resources.read_resource_bytes(&wav);
    match (o, w) {
        (Ok(_), Ok(_)) => Err(format!(
            "ambiguous clip '{sound}': both ogg and wav present"
        )),
        (Ok(ob), Err(e)) if is_not_found(&e) => {
            validate_audio_bytes(&ob)?;
            Ok(ogg)
        }
        (Err(e), Ok(wb)) if is_not_found(&e) => {
            validate_audio_bytes(&wb)?;
            Ok(wav)
        }
        (Err(a), Err(b)) if is_not_found(&a) && is_not_found(&b) => {
            Err(format!("missing clip '{}'", sound.as_str()))
        }
        (Err(e), _) if !is_not_found(&e) => Err(format!("read ogg: {e:?}")),
        (_, Err(e)) if !is_not_found(&e) => Err(format!("read wav: {e:?}")),
        _ => Err("unreadable clip".into()),
    }
}
pub fn validate_overlay_sounds(assets: &[OwnedAssets]) -> Result<(), String> {
    let mut count = 0usize;
    for a in assets {
        for (name, bytes) in &a.files {
            let s = name.as_str();
            if !(s.ends_with(".ogg") || s.ends_with(".wav")) {
                continue;
            }
            count += 1;
            if count > MAX_CLIPS {
                return Err(format!("too many clips (>{MAX_CLIPS})"));
            }
            validate_audio_bytes(bytes)?;
        }
    }
    Ok(())
}
