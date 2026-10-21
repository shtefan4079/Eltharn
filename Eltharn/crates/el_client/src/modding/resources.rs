use crate::render::model::BlockModel;
use crate::resources::manager::ClientResourceManager;
use el_core::resource_location::ResourceLocation;
use el_extension_manager::content_assets::{self, OwnedAssets};
use el_mod_content::ContentId;
use el_mod_content::catalog::Catalog;
use el_mod_content::spec::AssetKind;
use el_mod_runtime::wire::strict_json;
use el_platform_api::ResourceSnapshot;
use el_platform_api::{PlatformResult, VirtualPath};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;
const MOUNT_MAX_FILES: usize = 4096;
const MOUNT_MAX_BYTES: usize = 32 * 1024 * 1024;
const MAX_MODELS: usize = 1024;
const MAX_TEXTURES: usize = 1024;
const MAX_TEXTURE_BYTES: usize = 8 * 1024 * 1024;
const MAX_DECODED_BYTES: usize = 64 * 1024 * 1024;
const MAX_TEX_DIM: u32 = 2048;
const MAX_MCMETA: usize = 256 * 1024;
const MAX_ELEMENTS: usize = 512;
const MAX_QUADS: usize = 3072;
const MAX_PARENT_DEPTH: usize = 32;
const MAX_PARENT_NODES: usize = 1024;
pub fn image_dimensions(bytes: &[u8]) -> Result<(u32, u32), String> {
    if bytes.len() > MAX_TEXTURE_BYTES {
        return Err("PNG exceeds mod texture byte budget".into());
    }
    let dimensions =
        image::ImageReader::with_format(std::io::Cursor::new(bytes), image::ImageFormat::Png)
            .into_dimensions()
            .map_err(|e| format!("invalid mod PNG: {e}"))?;
    if dimensions.0 == 0
        || dimensions.1 == 0
        || dimensions.0 > MAX_TEX_DIM
        || dimensions.1 > MAX_TEX_DIM
    {
        return Err("mod PNG dimensions outside supported bounds".into());
    }
    Ok(dimensions)
}
struct OverlaySnapshot {
    files: BTreeMap<VirtualPath, Arc<Vec<u8>>>,
}
impl ResourceSnapshot for OverlaySnapshot {
    fn read(&self, path: &VirtualPath) -> PlatformResult<Option<Vec<u8>>> {
        Ok(self.files.get(path).map(|b| (**b).clone()))
    }
    fn read_shared(&self, path: &VirtualPath) -> PlatformResult<Option<Arc<Vec<u8>>>> {
        Ok(self.files.get(path).cloned())
    }
    fn list(&self, prefix: &VirtualPath) -> PlatformResult<Vec<VirtualPath>> {
        Ok(self
            .files
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }
}
pub fn mount(
    base: &ClientResourceManager,
    assets: &[OwnedAssets],
) -> Result<ClientResourceManager, String> {
    let mut total_files = 0usize;
    let mut total_bytes = 0usize;
    let mut decoded_bytes = 0usize;
    let mut image_count = 0usize;
    for pack in assets {
        for (path, bytes) in pack.files.iter() {
            total_files += 1;
            total_bytes += bytes.len();
            if total_files > MOUNT_MAX_FILES {
                return Err("too many overlay files".to_string());
            }
            if total_bytes > MOUNT_MAX_BYTES {
                return Err("overlay too large".to_string());
            }
            content_assets::validate_asset(&pack.owner, path, bytes)
                .map_err(|e| format!("{path}: {e}"))?;
            if path.ends_with(".png") {
                let (width, height) = image_dimensions(bytes)?;
                decoded_bytes += width as usize * height as usize * 4;
                image_count += 1;
                if decoded_bytes > MAX_DECODED_BYTES || image_count > MAX_TEXTURES {
                    return Err("mod overlay image budget exceeded".into());
                }
            }
            VirtualPath::new(path).map_err(|e| format!("{path}: {e:?}"))?;
        }
    }
    let mut map: BTreeMap<VirtualPath, Arc<Vec<u8>>> = BTreeMap::new();
    for pack in assets {
        for (path, bytes) in pack.files.iter() {
            let vp = VirtualPath::new(path).map_err(|e| format!("{path}: {e:?}"))?;
            if map.contains_key(&vp) {
                return Err(format!("duplicate asset path: {path}"));
            }
            map.insert(vp, Arc::new(bytes.clone()));
        }
    }
    let snapshot = OverlaySnapshot { files: map };
    base.clone()
        .with_prepared_resource_pack_snapshot(&snapshot)
        .map_err(|e| format!("mount overlay: {e:?}"))
}
pub struct ModelAssets {
    pub models: BTreeMap<ContentId, BlockModel>,
    pub materials: BTreeMap<ContentId, super::material::Material>,
    pub textures: Vec<ResourceLocation>,
}
pub fn load_models(
    resources: &ClientResourceManager,
    catalog: &Catalog,
) -> Result<ModelAssets, String> {
    let model_ids = collect_model_ids(catalog)?;
    if model_ids.len() > MAX_MODELS {
        return Err("too many models".to_string());
    }
    let mut models: BTreeMap<ContentId, BlockModel> = BTreeMap::new();
    let mut texture_lits: BTreeSet<(String, String)> = BTreeSet::new();
    let mut memo = HashMap::new();
    let mut nodes = HashSet::new();
    for id in model_ids.iter() {
        let loc = content_id_to_location(id)?;
        visit(resources, &loc, &mut memo, &mut Vec::new(), &mut nodes)?;
        let merged =
            BlockModel::load(resources, &loc).map_err(|e| format!("load model {loc:?}: {e:?}"))?;
        reject_material_metadata(resources, &loc)?;
        validate_geometry(id, &merged)?;
        let lits = collect_literals(&merged, id)?;
        for (ns, path) in lits {
            texture_lits.insert((ns, path));
        }
        if models.insert(id.clone(), merged).is_some() {
            return Err("duplicate model id".to_string());
        }
    }
    if texture_lits.len() > MAX_TEXTURES {
        return Err("too many textures".to_string());
    }
    let mut decoded_total: usize = 0;
    let mut decoded_dims: BTreeMap<(String, String), (u32, u32)> = BTreeMap::new();
    let mut textures: Vec<ResourceLocation> = Vec::new();
    for (ns, path) in texture_lits {
        let lit = ResourceLocation::new(&ns, &path);
        format!("{ns}:{path}")
            .parse::<ContentId>()
            .map_err(|e| format!("bad texture id {ns}:{path}: {e}"))?;
        let file = ResourceLocation::new(&ns, &format!("textures/{path}.png"));
        let bytes = resources
            .read_resource_bytes(&file)
            .map_err(|e| format!("missing texture {ns}:{path}: {e:?}"))?;
        if bytes.len() > MAX_TEXTURE_BYTES {
            return Err(format!("texture too large: {ns}:{path}"));
        }
        let (width, height) =
            image::ImageReader::with_format(std::io::Cursor::new(&bytes), image::ImageFormat::Png)
                .into_dimensions()
                .map_err(|e| format!("bad png header {ns}:{path}: {e}"))?;
        if width == 0 || height == 0 || width > MAX_TEX_DIM || height > MAX_TEX_DIM {
            return Err(format!("texture dimensions: {ns}:{path}"));
        }
        if decoded_total + width as usize * height as usize * 4 > MAX_DECODED_BYTES {
            return Err("decoded textures too large".into());
        }
        let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .map_err(|e| format!("bad png {ns}:{path}: {e}"))?;
        if img.width() > MAX_TEX_DIM || img.height() > MAX_TEX_DIM {
            return Err(format!("texture dimensions: {ns}:{path}"));
        }
        decoded_total += (img.width() as usize) * (img.height() as usize) * 4;
        if decoded_total > MAX_DECODED_BYTES {
            return Err("decoded textures too large".to_string());
        }
        decoded_dims.insert((ns.clone(), path.clone()), (img.width(), img.height()));
        decoded_total += check_mcmeta(resources, &ns, &path, img.width(), img.height())?;
        if decoded_total > MAX_DECODED_BYTES {
            return Err("animated textures exceed decoded budget".into());
        }
        textures.push(lit);
    }
    textures.sort_by(|a, b| (&a.namespace, &a.path).cmp(&(&b.namespace, &b.path)));
    {
        let mut explicit_quads: usize = 0;
        for model in models.values() {
            if let Some(elems) = model.elements.as_ref() {
                for el in elems {
                    explicit_quads = explicit_quads
                        .checked_add(el.faces.len())
                        .ok_or_else(|| "too many quads".to_string())?;
                }
            }
        }
        let mut generated_counts: BTreeMap<(u32, u32), usize> = BTreeMap::new();
        for model in models.values() {
            let has_elements = model.elements.as_ref().map_or(false, |e| !e.is_empty());
            if has_elements {
                continue;
            }
            let texmap = match model.textures.as_ref() {
                Some(m) => m,
                None => continue,
            };
            for i in 0..10 {
                let key = format!("layer{}", i);
                let raw = match texmap.get(key.as_str()) {
                    Some(r) => r,
                    None => continue,
                };
                let lit = resolve_literal(model, raw.as_str())?;
                let dims = match decoded_dims.get(&lit) {
                    Some(d) => *d,
                    None => {
                        return Err(format!("missing generated texture {}:{}", lit.0, lit.1));
                    }
                };
                let entry = generated_counts.entry(dims).or_insert(0);
                *entry = entry
                    .checked_add(1)
                    .ok_or_else(|| "too many quads".to_string())?;
            }
        }
        check_quad_budget(explicit_quads, generated_counts)?;
    }
    let materials = model_ids
        .iter()
        .map(|id| super::material::load(resources, id).map(|m| (id.clone(), m)))
        .collect::<Result<_, _>>()?;
    Ok(ModelAssets {
        models,
        materials,
        textures,
    })
}
const MAX_TOTAL_QUADS: usize = 125_000;
fn check_quad_budget(
    explicit_quad_count: usize,
    generated_uses: impl IntoIterator<Item = ((u32, u32), usize)>,
) -> Result<usize, String> {
    let mut total = explicit_quad_count;
    if total > MAX_TOTAL_QUADS {
        return Err(format!("too many quads: {total} > {MAX_TOTAL_QUADS}"));
    }
    for ((w, h), uses) in generated_uses {
        if uses == 0 {
            continue;
        }
        let pixels = (w as usize)
            .checked_mul(h as usize)
            .ok_or_else(|| "too many quads".to_string())?;
        let per_use = pixels
            .checked_mul(4)
            .and_then(|v| v.checked_add(2))
            .ok_or_else(|| "too many quads".to_string())?;
        let add = per_use
            .checked_mul(uses)
            .ok_or_else(|| "too many quads".to_string())?;
        total = total
            .checked_add(add)
            .ok_or_else(|| "too many quads".to_string())?;
        if total > MAX_TOTAL_QUADS {
            return Err(format!("too many quads: {total} > {MAX_TOTAL_QUADS}"));
        }
    }
    Ok(total)
}
fn collect_model_ids(catalog: &Catalog) -> Result<Vec<ContentId>, String> {
    let ids: BTreeSet<_> = catalog
        .entries()
        .flat_map(|owned| owned.spec.assets())
        .filter(|asset| asset.kind == AssetKind::Model)
        .map(|asset| asset.id)
        .collect();
    Ok(ids.into_iter().collect())
}
fn content_id_to_location(id: &ContentId) -> Result<ResourceLocation, String> {
    Ok(ResourceLocation::from_string(id.as_str()))
}
fn normalize_parent(raw: &str) -> Option<ResourceLocation> {
    if raw.starts_with("builtin/") {
        return None;
    }
    let base = ResourceLocation::from_string(raw);
    let path = if base.path.contains('/') {
        base.path.clone()
    } else {
        format!("block/{}", base.path)
    };
    Some(ResourceLocation::new(&base.namespace, &path))
}
fn read_strict(resources: &ClientResourceManager, id: &ResourceLocation) -> Result<Value, String> {
    let text = resources
        .read_model(id)
        .map_err(|e| format!("read model {id:?}: {e:?}"))?;
    strict_json(text.as_bytes(), MAX_MCMETA).map_err(|e| format!("strict json {id:?}: {e}"))
}
fn visit(
    resources: &ClientResourceManager,
    id: &ResourceLocation,
    memo: &mut HashMap<String, usize>,
    stack: &mut Vec<String>,
    nodes: &mut HashSet<String>,
) -> Result<usize, String> {
    let key = format!("{}:{}", id.namespace, id.path);
    if let Some(h) = memo.get(&key) {
        return Ok(*h);
    }
    if stack.contains(&key) {
        return Err(format!("model parent cycle: {key}"));
    }
    if nodes.len() >= MAX_PARENT_NODES {
        return Err("model parent graph too large".to_string());
    }
    if stack.len() > MAX_PARENT_DEPTH {
        return Err(format!("model parent depth: {key}"));
    }
    nodes.insert(key.clone());
    let value = read_strict(resources, id).map_err(|e| format!("missing parent {key}: {e}"))?;
    let height = match value.get("parent").and_then(Value::as_str) {
        None => 0,
        Some(raw) => match normalize_parent(raw) {
            None => 0,
            Some(pid) => {
                stack.push(key.clone());
                let child = visit(resources, &pid, memo, stack, nodes)?;
                stack.pop();
                let h = child + 1;
                if h > MAX_PARENT_DEPTH {
                    return Err(format!("model parent depth: {key}"));
                }
                h
            }
        },
    };
    memo.insert(key, height);
    Ok(height)
}
fn reject_material_metadata(
    resources: &ClientResourceManager,
    id: &ResourceLocation,
) -> Result<(), String> {
    let v = read_strict(resources, id)?;
    if let Value::Object(m) = &v {
        for k in ["shader", "shaders", "material", "materials"] {
            if m.contains_key(k) {
                return Err(format!("unsupported model metadata: {k}"));
            }
        }
    }
    Ok(())
}
fn validate_geometry(id: &ContentId, model: &BlockModel) -> Result<(), String> {
    let tag = serde_json::to_value(id).map_err(|e| format!("model id: {e}"))?;
    let elems = model.elements.as_ref();
    let count = elems.map(|v| v.len()).unwrap_or(0);
    if count > MAX_ELEMENTS {
        return Err(format!("too many elements: {tag}"));
    }
    let mut quads = 0usize;
    if let Some(list) = elems {
        for el in list {
            for (direction, face) in &el.faces {
                let valid_face =
                    |s: &str| matches!(s, "up" | "down" | "north" | "south" | "east" | "west");
                if !valid_face(direction)
                    || face.cullface.as_deref().is_some_and(|s| !valid_face(s))
                    || face
                        .rotation
                        .is_some_and(|r| !matches!(r, 0 | 90 | 180 | 270))
                    || face
                        .uv
                        .is_some_and(|uv| uv.iter().any(|v| !v.is_finite() || v.abs() > 4096.0))
                {
                    return Err(format!("invalid model face: {tag}"));
                }
            }
            quads += el.faces.len();
            if quads > MAX_QUADS {
                return Err(format!("too many quads: {tag}"));
            }
            for c in el.from.iter().chain(el.to.iter()) {
                if !c.is_finite() || *c < -16.0 || *c > 32.0 {
                    return Err(format!("bad bounds: {tag}"));
                }
            }
            if let Some(rot) = &el.rotation {
                if rot.axis != "x" && rot.axis != "y" && rot.axis != "z" {
                    return Err(format!("bad axis: {tag}"));
                }
                if !rot.angle.is_finite() {
                    return Err(format!("bad rotation: {tag}"));
                }
                for c in rot.origin {
                    if !c.is_finite() {
                        return Err(format!("bad origin: {tag}"));
                    }
                }
            }
        }
    }
    if let Some(display) = &model.display {
        for t in display.values() {
            for c in t
                .rotation
                .iter()
                .chain(t.translation.iter())
                .chain(t.scale.iter())
            {
                if !c.is_finite() {
                    return Err(format!("bad display: {tag}"));
                }
            }
        }
    }
    Ok(())
}
fn resolve_literal(model: &BlockModel, raw: &str) -> Result<(String, String), String> {
    let mut cur = raw.to_string();
    let mut seen: HashSet<String> = HashSet::new();
    for _ in 0..10 {
        if !cur.starts_with('#') {
            break;
        }
        let key = cur[1..].to_string();
        if key.is_empty() || !seen.insert(key.clone()) {
            return Err(format!("texture cycle: {raw}"));
        }
        let map = model
            .textures
            .as_ref()
            .ok_or_else(|| format!("missing ref: {raw}"))?;
        let next = map
            .get(&key)
            .ok_or_else(|| format!("missing ref: {raw}"))?
            .clone();
        cur = next;
    }
    if cur.starts_with('#') {
        return Err(format!("texture depth: {raw}"));
    }
    let loc = ResourceLocation::from_string(&cur);
    Ok((loc.namespace.clone(), loc.path.clone()))
}
fn collect_literals(model: &BlockModel, id: &ContentId) -> Result<Vec<(String, String)>, String> {
    let mut set: BTreeSet<(String, String)> = BTreeSet::new();
    if let Some(elems) = &model.elements {
        for el in elems {
            for face in el.faces.values() {
                set.insert(resolve_literal(model, &face.texture)?);
            }
        }
    }
    for i in 0..10 {
        let key = format!("layer{i}");
        let has = model
            .textures
            .as_ref()
            .is_some_and(|m| m.contains_key(&key));
        if has {
            set.insert(resolve_literal(model, &format!("#{key}"))?);
        }
    }
    let has_elements = model.elements.as_ref().is_some_and(|v| !v.is_empty());
    let has_layer0 = model
        .textures
        .as_ref()
        .is_some_and(|m| m.contains_key("layer0"));
    if !has_elements && !has_layer0 {
        let tag = serde_json::to_value(id).map_err(|e| format!("model id: {e}"))?;
        return Err(format!("unrenderable model: {tag}"));
    }
    Ok(set.into_iter().collect())
}
fn check_mcmeta(
    resources: &ClientResourceManager,
    ns: &str,
    path: &str,
    width: u32,
    height: u32,
) -> Result<usize, String> {
    let texture = ResourceLocation::new(ns, path);
    let bytes = match resources
        .read_texture_metadata(&texture)
        .map_err(|e| format!("mcmeta {ns}:{path}: {e}"))?
    {
        Some(b) => b,
        None => return Ok(0),
    };
    if bytes.len() > MAX_MCMETA {
        return Err(format!("mcmeta too large: {ns}:{path}"));
    }
    let text = String::from_utf8(bytes).map_err(|_| format!("bad mcmeta utf8: {ns}:{path}"))?;
    let v = strict_json(text.as_bytes(), MAX_MCMETA)
        .map_err(|e| format!("bad mcmeta {ns}:{path}: {e}"))?;
    if !v.is_object() {
        return Err(format!("bad mcmeta shape: {ns}:{path}"));
    }
    animation_bytes(&v, width, height)
}
fn animation_bytes(v: &Value, width: u32, height: u32) -> Result<usize, String> {
    let Some(animation) = v.get("animation") else {
        return Ok(0);
    };
    let animation = animation.as_object().ok_or("animation must be an object")?;
    let positive =
        |map: &serde_json::Map<String, Value>, key: &str, default: u64| -> Result<u64, String> {
            match map.get(key) {
                None => Ok(default),
                Some(v) => v
                    .as_u64()
                    .filter(|n| *n > 0 && *n <= u32::MAX as u64)
                    .ok_or_else(|| format!("invalid animation {key}")),
            }
        };
    let fw = positive(animation, "width", width as u64)?;
    let fh = positive(animation, "height", fw)?;
    if fw > width as u64 || fh > height as u64 || width as u64 % fw != 0 || height as u64 % fh != 0
    {
        return Err("animation frame dimensions do not divide texture".into());
    }
    let available = width as u64 / fw * (height as u64 / fh);
    let default_time = positive(animation, "frametime", 1)?;
    let count = if let Some(frames) = animation.get("frames") {
        let frames = frames
            .as_array()
            .ok_or("animation frames must be an array")?;
        if frames.is_empty() || frames.len() > 1024 {
            return Err("invalid animation frame count".into());
        }
        for frame in frames {
            let index = if let Some(detail) = frame.as_object() {
                positive(detail, "time", default_time)?;
                detail
                    .get("index")
                    .and_then(Value::as_u64)
                    .ok_or("invalid animation frame index")?
            } else {
                frame.as_u64().ok_or("invalid animation frame")?
            };
            if index >= available {
                return Err("animation frame index outside texture".into());
            }
        }
        frames.len() as u64
    } else {
        available
    };
    if count > 1024 {
        return Err("too many animation frames".into());
    }
    let bytes = fw * fh * 4 * count;
    if bytes > MAX_DECODED_BYTES as u64 {
        return Err("animation exceeds memory budget".into());
    }
    Ok(bytes as usize)
}
