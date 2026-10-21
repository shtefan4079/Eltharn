use crate::render::baked_model::Vertex;
use crate::render::mesher::ChunkMesh;
use crate::render::texture::{SpriteUvRect, TextureAtlas};
use crate::world::client_level::ClientLevel;
use base64::Engine;
use el_core::block_state::BlockState;
use el_core::resource_location::ResourceLocation;
use el_math::{Direction, block_pos::BlockPos};
use el_nbt::Tag;
use glam::{Mat4, Vec3};
use std::collections::HashMap;
pub struct DynamicPlayerSkullMesh {
    pub texture_key: String,
    pub mesh: ChunkMesh,
}
pub struct BuiltBlockEntityMesh {
    pub static_mesh: ChunkMesh,
    pub dynamic_player_skulls: Vec<DynamicPlayerSkullMesh>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlayerSkullProfile {
    pub name: Option<String>,
    pub uuid: Option<uuid::Uuid>,
    pub texture_url: Option<String>,
}
pub fn player_skull_profile(nbt: &[u8]) -> Option<PlayerSkullProfile> {
    let Ok(Tag::Compound(root)) = el_nbt::read_any_compound(nbt) else {
        return None;
    };
    let profile = el_nbt::compound_get(&root, "profile")
        .or_else(|| el_nbt::compound_get(&root, "SkullOwner"))
        .or_else(|| el_nbt::compound_get(&root, "Profile"))
        .or_else(|| el_nbt::compound_get(&root, "owner"))
        .or_else(|| el_nbt::compound_get(&root, "Owner"))
        .or_else(|| {
            el_nbt::compound_get(&root, "components").and_then(|c| match c {
                Tag::Compound(comp) => el_nbt::compound_get(comp, "minecraft:profile")
                    .or_else(|| el_nbt::compound_get(comp, "profile")),
                _ => None,
            })
        });
    let profile = match profile {
        Some(p) => p,
        None => {
            if el_nbt::compound_get(&root, "properties").is_some()
                || el_nbt::compound_get(&root, "Properties").is_some()
                || el_nbt::compound_get(&root, "id").is_some()
                || el_nbt::compound_get(&root, "Id").is_some()
                || el_nbt::compound_get(&root, "name").is_some()
                || el_nbt::compound_get(&root, "Name").is_some()
            {
                &Tag::Compound(root)
            } else {
                return None;
            }
        }
    };
    match profile {
        Tag::String(name) => {
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            Some(PlayerSkullProfile {
                name: Some(name.to_string()),
                uuid: None,
                texture_url: None,
            })
        }
        Tag::Compound(profile) => {
            let name = el_nbt::compound_get(profile, "name")
                .or_else(|| el_nbt::compound_get(profile, "Name"))
                .and_then(|tag| match tag {
                    Tag::String(s) => Some(s.clone()),
                    _ => None,
                });
            let uuid = el_nbt::compound_get(profile, "id")
                .or_else(|| el_nbt::compound_get(profile, "Id"))
                .and_then(parse_profile_uuid);
            let texture_url = profile_skin_url(profile);
            (uuid.is_some() || texture_url.is_some() || name.is_some()).then_some(
                PlayerSkullProfile {
                    name,
                    uuid,
                    texture_url,
                },
            )
        }
        _ => None,
    }
}
fn parse_profile_uuid(tag: &Tag) -> Option<uuid::Uuid> {
    match tag {
        Tag::String(value) => uuid::Uuid::parse_str(value).ok(),
        Tag::IntArray(values) if values.len() == 4 => {
            let mut bytes = [0_u8; 16];
            for (index, value) in values.iter().enumerate() {
                bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_be_bytes());
            }
            Some(uuid::Uuid::from_bytes(bytes))
        }
        _ => None,
    }
}
fn profile_skin_url(profile: &Vec<(String, Tag)>) -> Option<String> {
    let properties = el_nbt::compound_get(profile, "properties")
        .or_else(|| el_nbt::compound_get(profile, "Properties"))?;
    let mut encoded_values = Vec::new();
    match properties {
        Tag::List(values) => {
            for value in values {
                if let Tag::Compound(property) = value {
                    let prop_name = el_nbt::compound_get(property, "name")
                        .or_else(|| el_nbt::compound_get(property, "Name"));
                    if matches!(
                        prop_name, Some(Tag::String(name)) if name
                        .eq_ignore_ascii_case("textures")
                    ) {
                        if let Some(Tag::String(encoded)) = el_nbt::compound_get(property, "value")
                            .or_else(|| el_nbt::compound_get(property, "Value"))
                        {
                            encoded_values.push(encoded.as_str());
                        }
                    }
                }
            }
        }
        Tag::Compound(values) => {
            let textures_entry = el_nbt::compound_get(values, "textures")
                .or_else(|| el_nbt::compound_get(values, "Textures"));
            if let Some(value) = textures_entry {
                match value {
                    Tag::List(entries) => {
                        for entry in entries {
                            if let Tag::Compound(property) = entry
                                && let Some(Tag::String(encoded)) =
                                    el_nbt::compound_get(property, "value")
                                        .or_else(|| el_nbt::compound_get(property, "Value"))
                            {
                                encoded_values.push(encoded.as_str());
                            }
                        }
                    }
                    Tag::Compound(property) => {
                        if let Some(Tag::String(encoded)) = el_nbt::compound_get(property, "value")
                            .or_else(|| el_nbt::compound_get(property, "Value"))
                        {
                            encoded_values.push(encoded.as_str());
                        }
                    }
                    Tag::String(encoded) => {
                        encoded_values.push(encoded.as_str());
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
    for encoded in encoded_values {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .or_else(|_| base64::engine::general_purpose::STANDARD_NO_PAD.decode(encoded))
            .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(encoded))
            .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(encoded))
            .ok()?;
        let payload = serde_json::from_slice::<serde_json::Value>(&bytes).ok()?;
        if let Some(url) = payload
            .get("textures")
            .and_then(|textures| {
                textures
                    .get("SKIN")
                    .or_else(|| textures.get("skin"))
                    .or_else(|| Some(textures))
            })
            .and_then(|skin| skin.get("url").or_else(|| skin.get("URL")))
            .and_then(serde_json::Value::as_str)
        {
            return Some(url.to_owned());
        }
    }
    None
}
#[derive(Clone, Copy, Debug, Default)]
pub struct LidAnimation {
    pub openness: f32,
    pub old_openness: f32,
    pub target_open: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChestModelType {
    Single,
    Left,
    Right,
}
impl ChestModelType {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Single => Self::Single,
        }
    }
    fn connected_direction(self, facing: Direction) -> Option<Direction> {
        match self {
            Self::Left => Some(facing.clockwise()),
            Self::Right => Some(facing.counter_clockwise()),
            Self::Single => None,
        }
    }
}
fn chest_model_type(state: &BlockState) -> ChestModelType {
    let properties = crate::render::blockstate::effective_properties(state);
    match properties.get("type").map(String::as_str) {
        Some("left") => ChestModelType::Left,
        Some("right") => ChestModelType::Right,
        _ => ChestModelType::Single,
    }
}
fn chest_facing(state: &BlockState) -> Direction {
    crate::render::blockstate::effective_properties(state)
        .get("facing")
        .and_then(|value| Direction::from_name(value))
        .unwrap_or(Direction::North)
}
fn chest_combined_partner(
    level: &ClientLevel,
    pos: BlockPos,
    state: &BlockState,
    model_type: ChestModelType,
) -> Option<BlockPos> {
    let direction = model_type.connected_direction(chest_facing(state))?;
    let partner_pos = pos.relative(direction);
    if !level.has_chunk_at(partner_pos) {
        return None;
    }
    let partner_state = level.get_block_state(partner_pos);
    let valid_state_pair = partner_state.block == state.block
        && chest_model_type(&partner_state) == model_type.opposite()
        && chest_facing(&partner_state) == chest_facing(state);
    let has_partner_block_entity = level
        .chunk(partner_pos.x >> 4, partner_pos.z >> 4)
        .is_some_and(|chunk| chunk.block_entities.contains_key(&partner_pos));
    if chest_debug_enabled() && model_type != ChestModelType::Single {
        let line = format!(
            "[ChestDebug] function=chest_combined_partner pos=[{},{},{}] self={:?}/{:?} \
             partner_pos=[{},{},{}] partner_block={} partner_type={:?} partner_facing={:?} \
             valid_state_pair={} has_block_entity={}",
            pos.x,
            pos.y,
            pos.z,
            chest_facing(state),
            model_type,
            partner_pos.x,
            partner_pos.y,
            partner_pos.z,
            partner_state.block,
            chest_model_type(&partner_state),
            chest_facing(&partner_state),
            valid_state_pair,
            has_partner_block_entity,
        );
        log_chest_line_once(pos, line);
    }
    (valid_state_pair && has_partner_block_entity).then_some(partner_pos)
}
fn log_chest_line_once(pos: BlockPos, line: String) {
    if !chest_debug_enabled() {
        return;
    }
    static SEEN: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<BlockPos>>> =
        std::sync::OnceLock::new();
    let seen = SEEN.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    let Ok(mut seen) = seen.lock() else {
        return;
    };
    if !seen.insert(pos) {
        return;
    }
    eprintln!("{line}");
    if el_protocol_backend::diagnostics::file_enabled() {
        el_protocol_backend::diagnostics::write_line(&line);
    }
}
fn chest_open_amount(own: LidAnimation, partner: Option<LidAnimation>, partial_tick: f32) -> f32 {
    let own_open = own.interpolated(partial_tick);
    partner
        .map(|partner| own_open.max(partner.interpolated(partial_tick)))
        .unwrap_or(own_open)
}
#[derive(Clone, Copy, Debug)]
struct ChestRenderParameters {
    model_type: ChestModelType,
    combined_partner: Option<BlockPos>,
    open: f32,
}
fn chest_render_parameters(
    level: &ClientLevel,
    pos: BlockPos,
    state: &BlockState,
    animations: &HashMap<BlockPos, LidAnimation>,
    partial_tick: f32,
) -> ChestRenderParameters {
    let model_type = chest_model_type(state);
    let combined_partner = chest_combined_partner(level, pos, state, model_type);
    let own = animations.get(&pos).copied().unwrap_or_default();
    let partner = combined_partner.and_then(|partner_pos| animations.get(&partner_pos).copied());
    log_chest_state_once(pos, state, model_type, combined_partner);
    ChestRenderParameters {
        model_type,
        combined_partner,
        open: chest_open_amount(own, partner, partial_tick),
    }
}
fn chest_debug_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED
        .get_or_init(|| chest_debug_requested(std::env::var("ELTHARN_CHEST_DEBUG").ok().as_deref()))
}
fn chest_debug_requested(value: Option<&str>) -> bool {
    matches!(value, Some("1"))
}
fn log_chest_state_once(
    pos: BlockPos,
    state: &BlockState,
    model_type: ChestModelType,
    combined_partner: Option<BlockPos>,
) {
    if !chest_debug_enabled() {
        return;
    }
    static SEEN: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<BlockPos>>> =
        std::sync::OnceLock::new();
    let seen = SEEN.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()));
    let Ok(mut seen) = seen.lock() else {
        return;
    };
    if !seen.insert(pos) {
        return;
    }
    let properties = crate::render::blockstate::effective_properties(state);
    let raw_facing = state.properties.get("facing").map(String::as_str);
    let raw_type = state.properties.get("type").map(String::as_str);
    let line = format!(
        "[ChestDebug] function=chest_render_parameters pos=[{},{},{}] block={} state_id={} \
         raw_facing={:?} raw_type={:?} effective_facing={:?} effective_type={:?} \
         resolved_facing={:?} model_type={:?} partner={:?} texture={}",
        pos.x,
        pos.y,
        pos.z,
        state.block,
        state.id,
        raw_facing,
        raw_type,
        properties.get("facing").map(String::as_str),
        properties.get("type").map(String::as_str),
        chest_facing(state),
        model_type,
        combined_partner.map(|p| (p.x, p.y, p.z)),
        chest_texture(
            chest_material(state.block.path.as_str(), xmas_textures()),
            model_type
        ),
    );
    eprintln!("{line}");
    if el_protocol_backend::diagnostics::file_enabled() {
        el_protocol_backend::diagnostics::write_line(&line);
    }
}
impl LidAnimation {
    pub fn tick(&mut self) {
        self.old_openness = self.openness;
        let target = if self.target_open { 1.0 } else { 0.0 };
        self.openness += (target - self.openness).clamp(-0.1, 0.1);
    }
    fn interpolated(self, partial_tick: f32) -> f32 {
        self.old_openness + (self.openness - self.old_openness) * partial_tick
    }
    pub fn is_animating(self) -> bool {
        self.openness != self.old_openness
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PotWobbleStyle {
    Positive,
    Negative,
}
#[derive(Clone, Copy, Debug)]
pub struct PotWobbleAnimation {
    pub style: PotWobbleStyle,
    pub started_at_game_time: i64,
}
impl PotWobbleAnimation {
    pub fn from_event_parameter(parameter: u8, started_at_game_time: i64) -> Option<Self> {
        let style = match parameter {
            0 => PotWobbleStyle::Positive,
            1 => PotWobbleStyle::Negative,
            _ => return None,
        };
        Some(Self {
            style,
            started_at_game_time,
        })
    }
    fn progress_at(self, game_time: i64, partial_tick: f32) -> f32 {
        (game_time.wrapping_sub(self.started_at_game_time) as f32 + partial_tick)
            / f32::from(self.duration())
    }
    pub fn is_animating(self, game_time: i64, partial_tick: f32) -> bool {
        (0.0..=1.0).contains(&self.progress_at(game_time, partial_tick))
    }
    fn duration(self) -> u8 {
        match self.style {
            PotWobbleStyle::Positive => 7,
            PotWobbleStyle::Negative => 10,
        }
    }
    fn transform(self, game_time: i64, partial_tick: f32) -> Mat4 {
        let progress = self.progress_at(game_time, partial_tick);
        if !(0.0..=1.0).contains(&progress) {
            return Mat4::IDENTITY;
        }
        let pivot = Vec3::new(0.5, 0.0, 0.5);
        let around = |rotation: Mat4| {
            Mat4::from_translation(pivot) * rotation * Mat4::from_translation(-pivot)
        };
        match self.style {
            PotWobbleStyle::Positive => {
                let delta_time = progress * std::f32::consts::TAU;
                let tilt_x = -1.5 * (delta_time.cos() + 0.5) * (delta_time * 0.5).sin();
                let tilt_z = delta_time.sin();
                around(Mat4::from_rotation_x(tilt_x / 64.0))
                    * around(Mat4::from_rotation_z(tilt_z / 64.0))
            }
            PotWobbleStyle::Negative => {
                let angle =
                    (-progress * 3.0 * std::f32::consts::PI).sin() * 0.125 * (1.0 - progress);
                around(Mat4::from_rotation_y(angle))
            }
        }
    }
}
const BLOCK_ENTITY_VIEW_DISTANCE: f64 = 64.0;
pub(crate) fn should_render_block_entity_like_java(pos: BlockPos, camera_position: Vec3) -> bool {
    let dx = f64::from(camera_position.x) - (f64::from(pos.x) + 0.5);
    let dy = f64::from(camera_position.y) - (f64::from(pos.y) + 0.5);
    let dz = f64::from(camera_position.z) - (f64::from(pos.z) + 0.5);
    dx * dx + dy * dy + dz * dz < BLOCK_ENTITY_VIEW_DISTANCE * BLOCK_ENTITY_VIEW_DISTANCE
}
#[derive(Clone, Copy)]
pub struct BlockEntityMeshInputs<'a> {
    pub player_skull_texture_keys: &'a HashMap<BlockPos, String>,
    pub camera_position: Option<Vec3>,
}
impl<'a> BlockEntityMeshInputs<'a> {
    pub fn new(
        player_skull_texture_keys: &'a HashMap<BlockPos, String>,
        camera_position: Vec3,
    ) -> Self {
        Self {
            player_skull_texture_keys,
            camera_position: Some(camera_position),
        }
    }
}
impl<'a> From<&'a HashMap<BlockPos, String>> for BlockEntityMeshInputs<'a> {
    fn from(player_skull_texture_keys: &'a HashMap<BlockPos, String>) -> Self {
        Self {
            player_skull_texture_keys,
            camera_position: None,
        }
    }
}
#[allow(dead_code)]
pub fn build_mesh(
    level: &ClientLevel,
    atlas: &TextureAtlas,
    lid_animations: &HashMap<BlockPos, LidAnimation>,
    pot_wobbles: &HashMap<BlockPos, PotWobbleAnimation>,
    partial_tick: f32,
    camera_position: Option<Vec3>,
) -> ChunkMesh {
    let no_player_skulls = HashMap::new();
    build_mesh_with_dynamic_player_skulls(
        level,
        atlas,
        lid_animations,
        pot_wobbles,
        partial_tick,
        BlockEntityMeshInputs {
            player_skull_texture_keys: &no_player_skulls,
            camera_position,
        },
    )
    .static_mesh
}
pub fn build_mesh_with_dynamic_player_skulls<'a>(
    level: &ClientLevel,
    atlas: &TextureAtlas,
    lid_animations: &HashMap<BlockPos, LidAnimation>,
    pot_wobbles: &HashMap<BlockPos, PotWobbleAnimation>,
    partial_tick: f32,
    inputs: impl Into<BlockEntityMeshInputs<'a>>,
) -> BuiltBlockEntityMesh {
    let BlockEntityMeshInputs {
        player_skull_texture_keys,
        camera_position,
    } = inputs.into();
    let mut mesh = ChunkMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        translucent_indices: Vec::new(),
        visibility: {
            let mut visibility = crate::render::chunk::VisibilitySet::default();
            visibility.set_all(true);
            visibility
        },
    };
    let mut dynamic_player_skull_meshes: HashMap<String, ChunkMesh> = HashMap::new();
    for chunk in level.chunks.values() {
        for (&pos, block_entity) in &chunk.block_entities {
            if let Some(camera_position) = camera_position
                && !should_render_block_entity_like_java(pos, camera_position)
            {
                continue;
            }
            let state = level.get_block_state(pos);
            if state.block.path.ends_with("chest") {
                let chest =
                    chest_render_parameters(level, pos, &state, lid_animations, partial_tick);
                append_chest(&mut mesh, level, atlas, pos, &state, chest);
            } else if state.block.path.ends_with("shulker_box") {
                append_shulker_box(
                    &mut mesh,
                    level,
                    atlas,
                    pos,
                    &state,
                    lid_animations.get(&pos).copied().unwrap_or_default(),
                    partial_tick,
                );
            } else if state.block.path == "conduit" {
                append_conduit(&mut mesh, level, atlas, pos);
            } else if state.block.path == "decorated_pot" {
                append_decorated_pot(
                    &mut mesh,
                    level,
                    atlas,
                    pos,
                    &state,
                    &block_entity.nbt,
                    pot_wobbles.get(&pos).copied(),
                    partial_tick,
                );
            } else if state.block.path.ends_with("banner") {
                append_banner(
                    &mut mesh,
                    level,
                    atlas,
                    pos,
                    &state,
                    &block_entity.nbt,
                    partial_tick,
                );
            } else if is_skull_block(&state.block.path) {
                append_skull(
                    &mut mesh,
                    level,
                    atlas,
                    pos,
                    &state,
                    partial_tick,
                    player_skull_texture_keys.get(&pos),
                    &mut dynamic_player_skull_meshes,
                );
            } else if state.block.path.ends_with("copper_golem_statue") {
                append_copper_golem_statue(&mut mesh, level, atlas, pos, &state);
            }
        }
    }
    log_block_entity_mesh_summary(level);
    let dynamic_player_skulls = dynamic_player_skull_meshes
        .into_iter()
        .map(|(texture_key, mesh)| DynamicPlayerSkullMesh { texture_key, mesh })
        .collect();
    BuiltBlockEntityMesh {
        static_mesh: mesh,
        dynamic_player_skulls,
    }
}
fn log_block_entity_mesh_summary(level: &ClientLevel) {
    if !chest_debug_enabled() {
        return;
    }
    let chunks = level.chunks.len();
    let block_entities: usize = level.chunks.values().map(|c| c.block_entities.len()).sum();
    let chests = level
        .chunks
        .values()
        .flat_map(|c| c.block_entities.keys())
        .filter(|pos| level.get_block_state(**pos).block.path.ends_with("chest"))
        .count();
    static LAST: std::sync::OnceLock<std::sync::Mutex<Option<(usize, usize, usize)>>> =
        std::sync::OnceLock::new();
    let last = LAST.get_or_init(|| std::sync::Mutex::new(None));
    let Ok(mut last) = last.lock() else {
        return;
    };
    let now = (chunks, block_entities, chests);
    if *last == Some(now) {
        return;
    }
    *last = Some(now);
    let line = format!(
        "[ChestDebug] function=build_mesh summary chunks={chunks} \
         block_entities={block_entities} chests={chests}"
    );
    if el_protocol_backend::diagnostics::console_enabled() {
        println!("{line}");
    }
    if el_protocol_backend::diagnostics::file_enabled() {
        el_protocol_backend::diagnostics::write_line(&line);
    }
}
fn append_copper_golem_statue(
    mesh: &mut ChunkMesh,
    level: &ClientLevel,
    atlas: &TextureAtlas,
    pos: BlockPos,
    state: &BlockState,
) {
    let oxidation = if state.block.path.contains("oxidized") {
        "_oxidized"
    } else if state.block.path.contains("weathered") {
        "_weathered"
    } else if state.block.path.contains("exposed") {
        "_exposed"
    } else {
        ""
    };
    let texture = ResourceLocation::new(
        "minecraft",
        &format!("entity/copper_golem/copper_golem{oxidation}"),
    );
    let Some(sprite) = atlas.sprites.get(&texture).copied() else {
        return;
    };
    let facing = state
        .get_property("facing")
        .and_then(|facing| Direction::from_name(facing))
        .unwrap_or(Direction::North);
    let root = Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
        * Mat4::from_rotation_y(-direction_y_rotation(opposite_direction(facing)).to_radians())
        * Mat4::from_rotation_z(std::f32::consts::PI);
    let world = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
    let light = block_entity_light(level, pos);
    match state.get_property("pose").map(String::as_str) {
        Some("running") => append_running_copper_golem(mesh, sprite, root, world, light),
        Some("sitting") => append_sitting_copper_golem(mesh, sprite, root, world, light),
        Some("star") => append_star_copper_golem(mesh, sprite, root, world, light),
        _ => append_standing_copper_golem(mesh, sprite, root, world, light),
    }
}
fn statue_cube(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    transform: Mat4,
    world: Vec3,
    light: [f32; 2],
    uv: [f32; 2],
    origin: Vec3,
    size: Vec3,
    deformation: f32,
) {
    append_cube(
        mesh,
        sprite,
        uv,
        [64.0, 64.0],
        origin - Vec3::splat(deformation),
        size + Vec3::splat(deformation * 2.0),
        None,
        transform,
        world,
        light,
    );
}
fn append_golem_head(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    transform: Mat4,
    world: Vec3,
    light: [f32; 2],
    y_adjust: f32,
) {
    for (uv, origin, size, deformation) in [
        (
            [0.0, 0.0],
            Vec3::new(-4.0, -5.0 + y_adjust, -5.0),
            Vec3::new(8.0, 5.0, 10.0),
            0.0,
        ),
        (
            [56.0, 0.0],
            Vec3::new(-1.0, -2.0 + y_adjust, -6.0),
            Vec3::new(2.0, 3.0, 2.0),
            0.0,
        ),
        (
            [37.0, 8.0],
            Vec3::new(-1.0, -9.0 + y_adjust, -1.0),
            Vec3::new(2.0, 4.0, 2.0),
            -0.015,
        ),
        (
            [37.0, 0.0],
            Vec3::new(-2.0, -13.0 + y_adjust, -2.0),
            Vec3::splat(4.0),
            -0.015,
        ),
    ] {
        statue_cube(
            mesh,
            sprite,
            transform,
            world,
            light,
            uv,
            origin,
            size,
            deformation,
        );
    }
}
fn append_standing_copper_golem(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    root: Mat4,
    world: Vec3,
    light: [f32; 2],
) {
    let body = root * Mat4::from_translation(Vec3::new(0.0, -5.0, 0.0) / 16.0);
    statue_cube(
        mesh,
        sprite,
        body,
        world,
        light,
        [0.0, 15.0],
        Vec3::new(-4.0, -6.0, -3.0),
        Vec3::new(8.0, 6.0, 6.0),
        0.0,
    );
    append_golem_head(
        mesh,
        sprite,
        body * Mat4::from_translation(Vec3::new(0.0, -6.0, 0.0) / 16.0),
        world,
        light,
        0.0,
    );
    statue_cube(
        mesh,
        sprite,
        body * Mat4::from_translation(Vec3::new(-4.0, -6.0, 0.0) / 16.0),
        world,
        light,
        [36.0, 16.0],
        Vec3::new(-3.0, -1.0, -2.0),
        Vec3::new(3.0, 10.0, 4.0),
        0.0,
    );
    statue_cube(
        mesh,
        sprite,
        body * Mat4::from_translation(Vec3::new(4.0, -6.0, 0.0) / 16.0),
        world,
        light,
        [50.0, 16.0],
        Vec3::new(0.0, -1.0, -2.0),
        Vec3::new(3.0, 10.0, 4.0),
        0.0,
    );
    let legs = root * Mat4::from_translation(Vec3::new(0.0, -5.0, 0.0) / 16.0);
    statue_cube(
        mesh,
        sprite,
        legs,
        world,
        light,
        [0.0, 27.0],
        Vec3::new(-4.0, 0.0, -2.0),
        Vec3::new(4.0, 5.0, 4.0),
        0.0,
    );
    statue_cube(
        mesh,
        sprite,
        legs,
        world,
        light,
        [16.0, 27.0],
        Vec3::new(0.0, 0.0, -2.0),
        Vec3::new(4.0, 5.0, 4.0),
        0.0,
    );
}
fn part_pose(offset: Vec3, rotation: Vec3) -> Mat4 {
    Mat4::from_translation(offset / 16.0)
        * Mat4::from_rotation_z(rotation.z)
        * Mat4::from_rotation_y(rotation.y)
        * Mat4::from_rotation_x(rotation.x)
}
fn append_running_copper_golem(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    root: Mat4,
    world: Vec3,
    light: [f32; 2],
) {
    let body = root * part_pose(Vec3::new(-1.064, -5.0, 0.0), Vec3::ZERO);
    statue_cube(
        mesh,
        sprite,
        body * part_pose(
            Vec3::new(1.1, 0.1, 0.7),
            Vec3::new(0.1204, -0.0064, -0.0779),
        ),
        world,
        light,
        [0.0, 15.0],
        Vec3::new(-4.02, -6.116, -3.5),
        Vec3::new(8.0, 6.0, 6.0),
        0.0,
    );
    append_golem_head(
        mesh,
        sprite,
        body * part_pose(Vec3::new(0.7, -5.6, -1.8), Vec3::ZERO),
        world,
        light,
        -0.1,
    );
    statue_cube(
        mesh,
        sprite,
        body * part_pose(Vec3::new(-4.0, -6.0, 0.0), Vec3::ZERO)
            * part_pose(Vec3::new(0.7, -0.248, -1.62), Vec3::new(1.0036, 0.0, 0.0)),
        world,
        light,
        [36.0, 16.0],
        Vec3::new(-3.052, -1.11, -2.036),
        Vec3::new(3.0, 10.0, 4.0),
        0.0,
    );
    statue_cube(
        mesh,
        sprite,
        body * part_pose(Vec3::new(4.0, -6.0, 0.0), Vec3::ZERO)
            * part_pose(
                Vec3::new(0.732, 0.0, 0.0),
                Vec3::new(-0.8715, -0.0535, -0.0449),
            ),
        world,
        light,
        [50.0, 16.0],
        Vec3::new(0.032, -1.1, -2.0),
        Vec3::new(3.0, 10.0, 4.0),
        0.0,
    );
    statue_cube(
        mesh,
        sprite,
        root * part_pose(Vec3::new(-3.064, -5.0, 0.0), Vec3::ZERO)
            * part_pose(Vec3::new(1.048, 0.0, -0.9), Vec3::new(-0.8727, 0.0, 0.0)),
        world,
        light,
        [0.0, 27.0],
        Vec3::new(-1.856, -0.1, -1.09),
        Vec3::new(4.0, 5.0, 4.0),
        0.0,
    );
    statue_cube(
        mesh,
        sprite,
        root * part_pose(Vec3::new(0.936, -5.0, 0.0), Vec3::ZERO)
            * part_pose(Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.7854, 0.0, 0.0)),
        world,
        light,
        [16.0, 27.0],
        Vec3::new(-2.088, -0.1, -2.0),
        Vec3::new(4.0, 5.0, 4.0),
        0.0,
    );
}
fn append_sitting_copper_golem(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    root: Mat4,
    world: Vec3,
    light: [f32; 2],
) {
    let body = root * part_pose(Vec3::new(0.0, -3.0, 2.325), Vec3::ZERO);
    statue_cube(
        mesh,
        sprite,
        body,
        world,
        light,
        [3.0, 19.0],
        Vec3::new(-3.0, -4.0, -4.525),
        Vec3::new(6.0, 1.0, 6.0),
        0.0,
    );
    statue_cube(
        mesh,
        sprite,
        body,
        world,
        light,
        [0.0, 15.0],
        Vec3::new(-4.0, -3.0, -3.525),
        Vec3::new(8.0, 6.0, 6.0),
        0.0,
    );
    statue_cube(
        mesh,
        sprite,
        body * part_pose(
            Vec3::new(0.0, -1.0, -4.325),
            Vec3::new(0.0, 0.0, -std::f32::consts::PI),
        ),
        world,
        light,
        [3.0, 18.0],
        Vec3::new(-4.0, -3.0, -2.2),
        Vec3::new(8.0, 6.0, 3.0),
        0.0,
    );
    let head = body * part_pose(Vec3::new(0.0, -6.0, -0.2), Vec3::ZERO);
    for (uv, origin, size, deformation) in [
        (
            [37.0, 8.0],
            Vec3::new(-1.0, -7.0, -3.3),
            Vec3::new(2.0, 4.0, 2.0),
            -0.015,
        ),
        (
            [37.0, 0.0],
            Vec3::new(-2.0, -11.0, -4.3),
            Vec3::splat(4.0),
            -0.015,
        ),
        (
            [0.0, 0.0],
            Vec3::new(-4.0, -3.0, -7.325),
            Vec3::new(8.0, 5.0, 10.0),
            0.0,
        ),
        (
            [56.0, 0.0],
            Vec3::new(-1.0, 0.0, -8.325),
            Vec3::new(2.0, 3.0, 2.0),
            0.0,
        ),
    ] {
        statue_cube(
            mesh,
            sprite,
            head,
            world,
            light,
            uv,
            origin,
            size,
            deformation,
        );
    }
    for (left, offset) in [
        (false, Vec3::new(-4.0, -5.6, -1.8)),
        (true, Vec3::new(4.0, -5.6, -1.7)),
    ] {
        let arm = body
            * part_pose(offset, Vec3::new(0.4363, 0.0, 0.0))
            * part_pose(
                if left {
                    Vec3::new(0.0, -0.0015, -0.0808)
                } else {
                    Vec3::new(0.0, 0.0893, 0.1198)
                },
                Vec3::new(-1.0472, 0.0, 0.0),
            );
        statue_cube(
            mesh,
            sprite,
            arm,
            world,
            light,
            if left { [50.0, 16.0] } else { [36.0, 16.0] },
            if left {
                Vec3::new(0.075, -1.0443, -1.8997)
            } else {
                Vec3::new(-3.075, -0.9733, -1.9966)
            },
            Vec3::new(3.0, 10.0, 4.0),
            0.0,
        );
    }
    for (left, offset) in [
        (false, Vec3::new(-2.1, -2.1, -2.075)),
        (true, Vec3::new(2.0, -2.0, -2.075)),
    ] {
        let leg = root
            * part_pose(offset, Vec3::ZERO)
            * part_pose(
                if left {
                    Vec3::new(0.05, -2.0, 1.075)
                } else {
                    Vec3::new(0.05, -1.9, 1.075)
                },
                Vec3::new(-1.5708, 0.0, 0.0),
            );
        statue_cube(
            mesh,
            sprite,
            leg,
            world,
            light,
            if left { [16.0, 27.0] } else { [0.0, 27.0] },
            Vec3::new(-2.0, 0.975, 0.0),
            Vec3::new(4.0, 5.0, 4.0),
            0.0,
        );
    }
}
fn append_star_copper_golem(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    root: Mat4,
    world: Vec3,
    light: [f32; 2],
) {
    let body = root * part_pose(Vec3::new(0.0, -5.0, 0.0), Vec3::ZERO);
    statue_cube(
        mesh,
        sprite,
        body,
        world,
        light,
        [0.0, 15.0],
        Vec3::new(-4.0, -6.0, -3.0),
        Vec3::new(8.0, 6.0, 6.0),
        0.0,
    );
    append_golem_head(
        mesh,
        sprite,
        body * part_pose(Vec3::new(0.0, -6.0, 0.0), Vec3::ZERO),
        world,
        light,
        0.0,
    );
    for (left, z_rotation) in [(false, 1.9199), (true, -1.9199)] {
        let arm =
            body * part_pose(
                Vec3::new(if left { 4.0 } else { -4.0 }, -6.0, 0.0),
                Vec3::ZERO,
            ) * part_pose(
                Vec3::new(if left { -1.0 } else { 1.0 }, 1.0, 0.0),
                Vec3::new(0.0, 0.0, z_rotation),
            );
        statue_cube(
            mesh,
            sprite,
            arm,
            world,
            light,
            if left { [50.0, 16.0] } else { [36.0, 16.0] },
            Vec3::new(-1.5, -5.0, -2.0),
            Vec3::new(3.0, 10.0, 4.0),
            0.0,
        );
    }
    for (left, offset, z_rotation) in [
        (false, Vec3::new(-3.0, -5.0, 0.0), 0.2618),
        (true, Vec3::new(1.0, -5.0, 0.0), -0.2618),
    ] {
        let leg = root
            * part_pose(offset, Vec3::ZERO)
            * part_pose(
                Vec3::new(
                    if left { 1.65 } else { 0.35 },
                    2.0,
                    if left { 0.0 } else { 0.01 },
                ),
                Vec3::new(0.0, 0.0, z_rotation),
            );
        statue_cube(
            mesh,
            sprite,
            leg,
            world,
            light,
            if left { [16.0, 27.0] } else { [0.0, 27.0] },
            Vec3::new(-2.0, -2.5, -2.0),
            Vec3::new(4.0, 5.0, 4.0),
            0.0,
        );
    }
}
pub(crate) fn is_skull_block(path: &str) -> bool {
    matches!(
        path,
        "skeleton_skull"
            | "skeleton_wall_skull"
            | "creeper_head"
            | "creeper_wall_head"
            | "dragon_head"
            | "dragon_wall_head"
            | "zombie_head"
            | "zombie_wall_head"
            | "wither_skeleton_skull"
            | "wither_skeleton_wall_skull"
            | "player_head"
            | "player_wall_head"
            | "piglin_head"
            | "piglin_wall_head"
    )
}
fn empty_block_entity_mesh() -> ChunkMesh {
    let mut visibility = crate::render::chunk::VisibilitySet::default();
    visibility.set_all(true);
    ChunkMesh {
        vertices: Vec::new(),
        indices: Vec::new(),
        translucent_indices: Vec::new(),
        visibility,
    }
}
fn append_skull(
    mesh: &mut ChunkMesh,
    level: &ClientLevel,
    atlas: &TextureAtlas,
    pos: BlockPos,
    state: &BlockState,
    partial_tick: f32,
    dynamic_texture_key: Option<&String>,
    dynamic_meshes: &mut HashMap<String, ChunkMesh>,
) {
    let texture = skull_texture(&state.block.path);
    let sprite = texture
        .as_ref()
        .and_then(|t| atlas.sprites.get(t).copied())
        .unwrap_or(SpriteUvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        });
    append_skull_with_sprite(
        mesh,
        level,
        sprite,
        pos,
        state,
        partial_tick,
        dynamic_texture_key,
        dynamic_meshes,
    );
}
#[allow(clippy::too_many_arguments)]
fn append_skull_with_sprite(
    mesh: &mut ChunkMesh,
    level: &ClientLevel,
    sprite: SpriteUvRect,
    pos: BlockPos,
    state: &BlockState,
    partial_tick: f32,
    dynamic_texture_key: Option<&String>,
    dynamic_meshes: &mut HashMap<String, ChunkMesh>,
) {
    let is_wall = state.block.path.contains("_wall_");
    let root = if is_wall {
        let facing = state
            .get_property("facing")
            .and_then(|value| Direction::from_name(value))
            .unwrap_or(Direction::North);
        let (step_x, _, step_z) = facing.step();
        Mat4::from_translation(Vec3::new(
            0.5 - step_x as f32 * 0.25,
            0.25,
            0.5 - step_z as f32 * 0.25,
        )) * Mat4::from_rotation_y(-opposite_direction(facing).to_y_rotation().to_radians())
            * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
    } else {
        let segment = state
            .get_property("rotation")
            .and_then(|rotation| rotation.parse::<u8>().ok())
            .unwrap_or(0)
            & 15;
        Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
            * Mat4::from_rotation_y(-(f32::from(segment) * 22.5).to_radians())
            * Mat4::from_scale(Vec3::new(-1.0, -1.0, 1.0))
    };
    let world = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
    let light = block_entity_light(level, pos);
    let powered = state
        .get_property("powered")
        .is_some_and(|powered| powered == "true");
    let animation = if powered {
        level.game_time() as f32 + partial_tick
    } else {
        0.0
    };
    if state.block.path.contains("dragon") {
        append_dragon_head(mesh, sprite, root, world, light, animation);
    } else if state.block.path.contains("piglin") {
        append_piglin_head(mesh, sprite, root, world, light, animation);
    } else if state.block.path.contains("player")
        && let Some(texture_key) = dynamic_texture_key
    {
        let dynamic_sprite = SpriteUvRect {
            u0: 0.0,
            v0: 0.0,
            u1: 1.0,
            v1: 1.0,
        };
        let dynamic_mesh = dynamic_meshes
            .entry(texture_key.clone())
            .or_insert_with(empty_block_entity_mesh);
        append_cube(
            dynamic_mesh,
            dynamic_sprite,
            [0.0, 0.0],
            [64.0, 64.0],
            Vec3::new(-4.0, -8.0, -4.0),
            Vec3::splat(8.0),
            None,
            root,
            world,
            light,
        );
        append_skull_hat(dynamic_mesh, dynamic_sprite, root, world, light);
    } else {
        let humanoid = state.block.path.contains("zombie") || state.block.path.contains("player");
        append_cube(
            mesh,
            sprite,
            [0.0, 0.0],
            if humanoid { [64.0, 64.0] } else { [64.0, 32.0] },
            Vec3::new(-4.0, -8.0, -4.0),
            Vec3::splat(8.0),
            None,
            root,
            world,
            light,
        );
        if humanoid {
            append_skull_hat(mesh, sprite, root, world, light);
        }
    }
}
fn append_skull_hat(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    root: Mat4,
    world: Vec3,
    light: [f32; 2],
) {
    let center = Vec3::new(0.0, -4.0, 0.0) / 16.0;
    let hat_transform = root
        * Mat4::from_translation(center)
        * Mat4::from_scale(Vec3::splat(8.5 / 8.0))
        * Mat4::from_translation(-center);
    append_cube(
        mesh,
        sprite,
        [32.0, 0.0],
        [64.0, 64.0],
        Vec3::new(-4.0, -8.0, -4.0),
        Vec3::splat(8.0),
        None,
        hat_transform,
        world,
        light,
    );
}
fn append_dragon_head(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    root: Mat4,
    world: Vec3,
    light: [f32; 2],
    animation: f32,
) {
    let head = root
        * Mat4::from_translation(Vec3::new(0.0, -7.986_666 / 16.0, 0.0))
        * Mat4::from_scale(Vec3::splat(0.75));
    for (uv, origin, size) in [
        (
            [176.0, 44.0],
            Vec3::new(-6.0, -1.0, -24.0),
            Vec3::new(12.0, 5.0, 16.0),
        ),
        (
            [112.0, 30.0],
            Vec3::new(-8.0, -8.0, -10.0),
            Vec3::splat(16.0),
        ),
        (
            [0.0, 0.0],
            Vec3::new(-5.0, -12.0, -4.0),
            Vec3::new(2.0, 4.0, 6.0),
        ),
        (
            [112.0, 0.0],
            Vec3::new(-5.0, -3.0, -22.0),
            Vec3::new(2.0, 2.0, 4.0),
        ),
        (
            [0.0, 0.0],
            Vec3::new(3.0, -12.0, -4.0),
            Vec3::new(2.0, 4.0, 6.0),
        ),
        (
            [112.0, 0.0],
            Vec3::new(3.0, -3.0, -22.0),
            Vec3::new(2.0, 2.0, 4.0),
        ),
    ] {
        append_cube(
            mesh,
            sprite,
            uv,
            [256.0, 256.0],
            origin,
            size,
            None,
            head,
            world,
            light,
        );
    }
    let jaw_angle = ((animation * std::f32::consts::PI * 0.2).sin() + 1.0) * 0.2;
    let jaw = head
        * Mat4::from_translation(Vec3::new(0.0, 4.0 / 16.0, -8.0 / 16.0))
        * Mat4::from_rotation_x(jaw_angle);
    append_cube(
        mesh,
        sprite,
        [176.0, 65.0],
        [256.0, 256.0],
        Vec3::new(-6.0, 0.0, -16.0),
        Vec3::new(12.0, 4.0, 16.0),
        None,
        jaw,
        world,
        light,
    );
}
fn append_piglin_head(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    root: Mat4,
    world: Vec3,
    light: [f32; 2],
    animation: f32,
) {
    for (uv, origin, size) in [
        (
            [0.0, 0.0],
            Vec3::new(-5.0, -8.0, -4.0),
            Vec3::new(10.0, 8.0, 8.0),
        ),
        (
            [31.0, 1.0],
            Vec3::new(-2.0, -4.0, -5.0),
            Vec3::new(4.0, 4.0, 1.0),
        ),
        (
            [2.0, 4.0],
            Vec3::new(2.0, -2.0, -5.0),
            Vec3::new(1.0, 2.0, 1.0),
        ),
        (
            [2.0, 0.0],
            Vec3::new(-3.0, -2.0, -5.0),
            Vec3::new(1.0, 2.0, 1.0),
        ),
    ] {
        append_cube(
            mesh,
            sprite,
            uv,
            [64.0, 64.0],
            origin,
            size,
            None,
            root,
            world,
            light,
        );
    }
    let left_angle = -((animation * std::f32::consts::PI * 0.2 * 1.2).cos() + 2.5) * 0.2;
    let right_angle = ((animation * std::f32::consts::PI * 0.2).cos() + 2.5) * 0.2;
    let left = root
        * Mat4::from_translation(Vec3::new(4.5, -6.0, 0.0) / 16.0)
        * Mat4::from_rotation_z(left_angle);
    let right = root
        * Mat4::from_translation(Vec3::new(-4.5, -6.0, 0.0) / 16.0)
        * Mat4::from_rotation_z(right_angle);
    append_cube(
        mesh,
        sprite,
        [51.0, 6.0],
        [64.0, 64.0],
        Vec3::new(0.0, 0.0, -2.0),
        Vec3::new(1.0, 5.0, 4.0),
        None,
        left,
        world,
        light,
    );
    append_cube(
        mesh,
        sprite,
        [39.0, 6.0],
        [64.0, 64.0],
        Vec3::new(-1.0, 0.0, -2.0),
        Vec3::new(1.0, 5.0, 4.0),
        None,
        right,
        world,
        light,
    );
}
fn skull_texture(path: &str) -> Option<ResourceLocation> {
    let texture = if path.contains("wither_skeleton") {
        "entity/skeleton/wither_skeleton"
    } else if path.contains("skeleton") {
        "entity/skeleton/skeleton"
    } else if path.contains("zombie") {
        "entity/zombie/zombie"
    } else if path.contains("creeper") {
        "entity/creeper/creeper"
    } else if path.contains("dragon") {
        "entity/enderdragon/dragon"
    } else if path.contains("piglin") {
        "entity/piglin/piglin"
    } else if path.contains("player") {
        "entity/player/wide/steve"
    } else {
        return None;
    };
    Some(ResourceLocation::new("minecraft", texture))
}
fn opposite_direction(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}
trait DirectionRotation {
    fn to_y_rotation(self) -> f32;
}
impl DirectionRotation for Direction {
    fn to_y_rotation(self) -> f32 {
        direction_y_rotation(self)
    }
}
fn block_entity_light(level: &ClientLevel, pos: BlockPos) -> [f32; 2] {
    let above = BlockPos::new(pos.x, pos.y + 1, pos.z);
    [
        f32::from(
            level
                .brightness(false, pos)
                .max(level.brightness(false, above)),
        ) / 16.0,
        f32::from(
            level
                .brightness(true, pos)
                .max(level.brightness(true, above)),
        ) / 16.0,
    ]
}
fn append_banner(
    mesh: &mut ChunkMesh,
    level: &ClientLevel,
    atlas: &TextureAtlas,
    pos: BlockPos,
    state: &BlockState,
    nbt: &[u8],
    partial_tick: f32,
) {
    let body_texture = ResourceLocation::new("minecraft", "entity/banner/banner_base");
    let base_pattern_texture = ResourceLocation::new("minecraft", "entity/banner/base");
    let (Some(body_sprite), Some(base_pattern_sprite)) = (
        atlas.sprites.get(&body_texture).copied(),
        atlas.sprites.get(&base_pattern_texture).copied(),
    ) else {
        return;
    };
    let is_wall = state.block.path.ends_with("_wall_banner");
    let angle_degrees = if is_wall {
        state
            .get_property("facing")
            .and_then(|facing| Direction::from_name(facing))
            .map(direction_y_rotation)
            .unwrap_or(0.0)
    } else {
        state
            .get_property("rotation")
            .and_then(|rotation| rotation.parse::<u8>().ok())
            .map(|segment| f32::from(segment & 15) * 22.5)
            .unwrap_or(0.0)
    };
    let root = Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
        * Mat4::from_rotation_y(-angle_degrees.to_radians())
        * Mat4::from_scale(Vec3::new(2.0 / 3.0, -2.0 / 3.0, -2.0 / 3.0));
    let world = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
    let above = BlockPos::new(pos.x, pos.y + 1, pos.z);
    let light = [
        f32::from(
            level
                .brightness(false, pos)
                .max(level.brightness(false, above)),
        ) / 16.0,
        f32::from(
            level
                .brightness(true, pos)
                .max(level.brightness(true, above)),
        ) / 16.0,
    ];
    if !is_wall {
        append_cube(
            mesh,
            body_sprite,
            [44.0, 0.0],
            [64.0, 64.0],
            Vec3::new(-1.0, -42.0, -1.0),
            Vec3::new(2.0, 42.0, 2.0),
            None,
            root,
            world,
            light,
        );
    }
    append_cube(
        mesh,
        body_sprite,
        [0.0, 42.0],
        [64.0, 64.0],
        if is_wall {
            Vec3::new(-10.0, -20.5, 9.5)
        } else {
            Vec3::new(-10.0, -44.0, -1.0)
        },
        Vec3::new(20.0, 2.0, 2.0),
        None,
        root,
        world,
        light,
    );
    let phase =
        ((i64::from(pos.x) * 7 + i64::from(pos.y) * 9 + i64::from(pos.z) * 13 + level.game_time())
            .rem_euclid(100) as f32
            + partial_tick)
            / 100.0;
    let flag_rotation =
        (-0.0125 + 0.01 * (std::f32::consts::TAU * phase).cos()) * std::f32::consts::PI;
    let flag_origin = if is_wall {
        Vec3::new(0.0, -20.5, 10.5)
    } else {
        Vec3::new(0.0, -44.0, 0.0)
    };
    let flag_transform =
        root * Mat4::from_translation(flag_origin / 16.0) * Mat4::from_rotation_x(flag_rotation);
    let flag_cube_origin = Vec3::new(-10.0, 0.0, -2.0);
    let flag_cube_size = Vec3::new(20.0, 40.0, 1.0);
    append_cube(
        mesh,
        body_sprite,
        [0.0, 0.0],
        [64.0, 64.0],
        flag_cube_origin,
        flag_cube_size,
        None,
        flag_transform,
        world,
        light,
    );
    let base_color = banner_color_from_block(&state.block.path);
    append_colored_cube_to(
        &mut mesh.vertices,
        base_pattern_sprite,
        [0.0, 0.0],
        [64.0, 64.0],
        flag_cube_origin,
        flag_cube_size,
        None,
        flag_transform,
        world,
        light,
        base_color,
        &mut mesh.translucent_indices,
    );
    for (layer, (texture, color)) in banner_patterns(nbt).into_iter().take(16).enumerate() {
        let Some(sprite) = atlas.sprites.get(&texture).copied() else {
            log::warn!(
                "[Banner] Pattern sprite not found in atlas: {:?} (layer {})",
                texture,
                layer
            );
            continue;
        };
        append_colored_cube_to(
            &mut mesh.vertices,
            sprite,
            [0.0, 0.0],
            [64.0, 64.0],
            flag_cube_origin,
            flag_cube_size,
            None,
            flag_transform,
            world,
            light,
            color,
            &mut mesh.translucent_indices,
        );
    }
}
fn direction_y_rotation(direction: Direction) -> f32 {
    match direction {
        Direction::South => 0.0,
        Direction::West => 90.0,
        Direction::North => 180.0,
        Direction::East => 270.0,
        Direction::Down | Direction::Up => 0.0,
    }
}
fn banner_color_from_block(path: &str) -> [f32; 4] {
    let color = path
        .strip_suffix("_wall_banner")
        .or_else(|| path.strip_suffix("_banner"))
        .unwrap_or("white");
    dye_color(color)
}
fn banner_patterns(nbt: &[u8]) -> Vec<(ResourceLocation, [f32; 4])> {
    let Ok(Tag::Compound(root)) = el_nbt::read_any_compound(nbt) else {
        return Vec::new();
    };
    let Some(Tag::List(patterns)) = el_nbt::compound_get(&root, "patterns") else {
        return Vec::new();
    };
    patterns
        .iter()
        .filter_map(|pattern| {
            let Tag::Compound(fields) = pattern else {
                return None;
            };
            let asset = match el_nbt::compound_get(fields, "pattern")? {
                Tag::String(id) => id.as_str(),
                Tag::Compound(direct) => match el_nbt::compound_get(direct, "asset_id")? {
                    Tag::String(id) => id.as_str(),
                    _ => return None,
                },
                _ => return None,
            };
            let color = match el_nbt::compound_get(fields, "color")? {
                Tag::String(color) => color.as_str(),
                Tag::Int(id) => dye_color_name_by_id(*id as u8),
                Tag::Byte(id) => dye_color_name_by_id(*id as u8),
                _ => return None,
            };
            let (namespace, path) = asset.split_once(':').unwrap_or(("minecraft", asset));
            Some((
                ResourceLocation::new(namespace, &format!("entity/banner/{path}")),
                dye_color(color),
            ))
        })
        .collect()
}
fn dye_color_name_by_id(id: u8) -> &'static str {
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
fn dye_color(name: &str) -> [f32; 4] {
    let rgb = match name {
        "white" => 0xf9_ff_fe,
        "orange" => 0xf9_80_1d,
        "magenta" => 0xc7_4e_bd,
        "light_blue" => 0x3a_b3_da,
        "yellow" => 0xfe_d8_3d,
        "lime" => 0x80_c7_1f,
        "pink" => 0xf3_8b_aa,
        "gray" => 0x47_4f_52,
        "light_gray" => 0x9d_9d_97,
        "cyan" => 0x16_9c_9c,
        "purple" => 0x89_32_b8,
        "blue" => 0x3c_44_aa,
        "brown" => 0x83_54_32,
        "green" => 0x5e_7c_16,
        "red" => 0xb0_2e_26,
        "black" => 0x1d_1d_21,
        _ => 0xfa_fa_fe,
    };
    [
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
        1.0,
    ]
}
fn append_decorated_pot(
    mesh: &mut ChunkMesh,
    level: &ClientLevel,
    atlas: &TextureAtlas,
    pos: BlockPos,
    state: &BlockState,
    nbt: &[u8],
    wobble: Option<PotWobbleAnimation>,
    partial_tick: f32,
) {
    let base_texture =
        ResourceLocation::new("minecraft", "entity/decorated_pot/decorated_pot_base");
    let side_texture =
        ResourceLocation::new("minecraft", "entity/decorated_pot/decorated_pot_side");
    let (Some(base_sprite), Some(side_sprite)) = (
        atlas.sprites.get(&base_texture).copied(),
        atlas.sprites.get(&side_texture).copied(),
    ) else {
        return;
    };
    let facing = state
        .get_property("facing")
        .and_then(|facing| Direction::from_name(facing))
        .unwrap_or(Direction::North);
    let root = pot_rotation(facing)
        * wobble
            .map(|animation| animation.transform(level.game_time(), partial_tick))
            .unwrap_or(Mat4::IDENTITY);
    let world = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
    let light = decorated_pot_light(level, pos);
    let neck = root
        * Mat4::from_translation(Vec3::new(0.0, 37.0 / 16.0, 1.0))
        * Mat4::from_rotation_x(std::f32::consts::PI);
    let (neck_origin, neck_dimensions, rim_origin, rim_dimensions) = decorated_pot_neck_geometry();
    append_cube(
        mesh,
        base_sprite,
        [0.0, 0.0],
        [32.0, 32.0],
        neck_origin,
        neck_dimensions,
        None,
        neck,
        world,
        light,
    );
    append_cube(
        mesh,
        base_sprite,
        [0.0, 5.0],
        [32.0, 32.0],
        rim_origin,
        rim_dimensions,
        None,
        neck,
        world,
        light,
    );
    for y in [0.0, 16.0] {
        append_cube(
            mesh,
            base_sprite,
            [-14.0, 13.0],
            [32.0, 32.0],
            Vec3::new(1.0, y, 1.0),
            Vec3::new(14.0, 0.0, 14.0),
            None,
            root,
            world,
            light,
        );
    }
    let decorations = decorated_pot_textures(nbt);
    let side_positions = [
        Vec3::new(14.0 / 16.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(14.0 / 16.0, 1.0, 0.0),
    ];
    let side_uvs = [[1.0, 0.0], [15.0, 0.0], [15.0, 16.0], [1.0, 16.0]];
    let side_parts = [
        (
            3usize,
            Mat4::from_translation(Vec3::new(1.0 / 16.0, 1.0, 15.0 / 16.0))
                * Mat4::from_rotation_x(std::f32::consts::PI),
        ),
        (
            0usize,
            Mat4::from_translation(Vec3::new(15.0 / 16.0, 1.0, 1.0 / 16.0))
                * Mat4::from_rotation_z(std::f32::consts::PI),
        ),
        (
            1usize,
            Mat4::from_translation(Vec3::new(1.0 / 16.0, 1.0, 1.0 / 16.0))
                * Mat4::from_rotation_z(std::f32::consts::PI)
                * Mat4::from_rotation_y(-std::f32::consts::FRAC_PI_2),
        ),
        (
            2usize,
            Mat4::from_translation(Vec3::new(15.0 / 16.0, 1.0, 15.0 / 16.0))
                * Mat4::from_rotation_z(std::f32::consts::PI)
                * Mat4::from_rotation_y(std::f32::consts::FRAC_PI_2),
        ),
    ];
    for (decoration_index, part_transform) in side_parts {
        let texture = &decorations[decoration_index];
        let sprite = atlas.sprites.get(texture).copied().unwrap_or(side_sprite);
        append_quad(
            mesh,
            sprite,
            side_positions,
            side_uvs,
            [16.0, 16.0],
            Direction::North,
            root * part_transform,
            world,
            light,
        );
    }
}
fn decorated_pot_neck_geometry() -> (Vec3, Vec3, Vec3, Vec3) {
    (
        Vec3::new(4.1, 17.1, 4.1),
        Vec3::new(7.8, 2.8, 7.8),
        Vec3::new(4.8, 19.8, 4.8),
        Vec3::new(6.4, 1.4, 6.4),
    )
}
fn decorated_pot_light(level: &ClientLevel, pos: BlockPos) -> [f32; 2] {
    [
        f32::from(level.brightness(false, pos)) / 16.0,
        f32::from(level.brightness(true, pos)) / 16.0,
    ]
}
fn decorated_pot_textures(nbt: &[u8]) -> [ResourceLocation; 4] {
    let fallback = || ResourceLocation::new("minecraft", "entity/decorated_pot/decorated_pot_side");
    let mut result = core::array::from_fn(|_| fallback());
    let Ok(Tag::Compound(root)) = el_nbt::read_any_compound(nbt) else {
        return result;
    };
    let Some(Tag::List(sherds)) = el_nbt::compound_get(&root, "sherds") else {
        return result;
    };
    for (index, sherd) in sherds.iter().take(4).enumerate() {
        if let Tag::String(item) = sherd
            && let Some(texture) = decorated_pot_pattern_texture(item)
        {
            result[index] = texture;
        }
    }
    result
}
fn decorated_pot_pattern_texture(item: &str) -> Option<ResourceLocation> {
    let pattern = el_core::item::decorated_pot_pattern_asset(item)?;
    Some(ResourceLocation::new(
        "minecraft",
        &format!("entity/decorated_pot/{pattern}"),
    ))
}
fn pot_rotation(facing: Direction) -> Mat4 {
    let angle = match facing {
        Direction::North => 0.0,
        Direction::West => std::f32::consts::FRAC_PI_2,
        Direction::South => std::f32::consts::PI,
        Direction::East => -std::f32::consts::FRAC_PI_2,
        Direction::Down | Direction::Up => 0.0,
    };
    Mat4::from_translation(Vec3::splat(0.5))
        * Mat4::from_rotation_y(angle)
        * Mat4::from_translation(Vec3::splat(-0.5))
}
fn append_conduit(mesh: &mut ChunkMesh, level: &ClientLevel, atlas: &TextureAtlas, pos: BlockPos) {
    let texture = ResourceLocation::new("minecraft", "entity/conduit/base");
    let Some(sprite) = atlas.sprites.get(&texture).copied() else {
        return;
    };
    let above = BlockPos::new(pos.x, pos.y + 1, pos.z);
    let light = [
        f32::from(
            level
                .brightness(false, pos)
                .max(level.brightness(false, above)),
        ) / 16.0,
        f32::from(
            level
                .brightness(true, pos)
                .max(level.brightness(true, above)),
        ) / 16.0,
    ];
    append_cube(
        mesh,
        sprite,
        [0.0, 0.0],
        [32.0, 16.0],
        Vec3::new(-3.0, -3.0, -3.0),
        Vec3::splat(6.0),
        None,
        Mat4::from_translation(Vec3::splat(0.5)),
        Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32),
        light,
    );
}
fn append_shulker_box(
    mesh: &mut ChunkMesh,
    level: &ClientLevel,
    atlas: &TextureAtlas,
    pos: BlockPos,
    state: &BlockState,
    animation: LidAnimation,
    partial_tick: f32,
) {
    let texture = shulker_texture(state.block.path.as_str());
    let Some(sprite) = atlas.sprites.get(&texture).copied() else {
        return;
    };
    let facing = state
        .get_property("facing")
        .and_then(|facing| Direction::from_name(facing))
        .unwrap_or(Direction::Up);
    let root = shulker_rotation(facing);
    let progress = animation.interpolated(partial_tick).clamp(0.0, 1.0);
    let world = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
    let above = BlockPos::new(pos.x, pos.y + 1, pos.z);
    let light = [
        f32::from(
            level
                .brightness(false, pos)
                .max(level.brightness(false, above)),
        ) / 16.0,
        f32::from(
            level
                .brightness(true, pos)
                .max(level.brightness(true, above)),
        ) / 16.0,
    ];
    let base = root * Mat4::from_translation(Vec3::new(0.0, 24.0 / 16.0, 0.0));
    append_cube(
        mesh,
        sprite,
        [0.0, 28.0],
        [64.0, 64.0],
        Vec3::new(-8.0, -8.0, -8.0),
        Vec3::new(16.0, 8.0, 16.0),
        None,
        base,
        world,
        light,
    );
    let lid = root
        * Mat4::from_translation(Vec3::new(0.0, (24.0 - progress * 8.0) / 16.0, 0.0))
        * Mat4::from_rotation_y(progress * 1.5 * std::f32::consts::PI);
    append_cube(
        mesh,
        sprite,
        [0.0, 0.0],
        [64.0, 64.0],
        Vec3::new(-8.0, -16.0, -8.0),
        Vec3::new(16.0, 12.0, 16.0),
        None,
        lid,
        world,
        light,
    );
}
fn shulker_texture(path: &str) -> ResourceLocation {
    let color = path.strip_suffix("_shulker_box").unwrap_or("");
    let texture = if color.is_empty() || color == "shulker" {
        "shulker".to_string()
    } else {
        format!("shulker_{color}")
    };
    ResourceLocation::new("minecraft", &format!("entity/shulker/{texture}"))
}
fn shulker_rotation(facing: Direction) -> Mat4 {
    let direction = direction_vector(facing);
    let rotation = glam::Quat::from_rotation_arc(Vec3::Y, direction);
    Mat4::from_translation(Vec3::splat(0.5))
        * Mat4::from_scale(Vec3::splat(0.9995))
        * Mat4::from_quat(rotation)
        * Mat4::from_scale(Vec3::new(1.0, -1.0, -1.0))
        * Mat4::from_translation(Vec3::new(0.0, -1.0, 0.0))
}
fn append_chest(
    mesh: &mut ChunkMesh,
    level: &ClientLevel,
    atlas: &TextureAtlas,
    pos: BlockPos,
    state: &BlockState,
    chest: ChestRenderParameters,
) {
    let material = chest_material(state.block.path.as_str(), xmas_textures());
    let texture = chest_texture(material, chest.model_type);
    let Some(sprite) = atlas.sprites.get(&texture).copied() else {
        return;
    };
    let facing = chest_facing(state);
    let rotation = chest_rotation(facing);
    let open = chest.open.clamp(0.0, 1.0);
    let eased_open = 1.0 - (1.0 - open).powi(3);
    let lid_rotation = -eased_open * std::f32::consts::FRAC_PI_2;
    let animated_part = rotation
        * Mat4::from_translation(Vec3::new(0.0, 9.0 / 16.0, 1.0 / 16.0))
        * Mat4::from_rotation_x(lid_rotation);
    let world = Vec3::new(pos.x as f32, pos.y as f32, pos.z as f32);
    let light_positions = chest
        .combined_partner
        .map(|partner_pos| [pos, partner_pos])
        .unwrap_or([pos, pos]);
    let light = [
        f32::from(
            level
                .brightness(false, light_positions[0])
                .max(level.brightness(false, light_positions[1])),
        ) / 16.0,
        f32::from(
            level
                .brightness(true, light_positions[0])
                .max(level.brightness(true, light_positions[1])),
        ) / 16.0,
    ];
    let (body_origin_x, body_width, hidden_face, lock_origin_x, lock_width) = match chest.model_type
    {
        ChestModelType::Left => (0.0, 15.0, Some(Direction::West), 0.0, 1.0),
        ChestModelType::Right => (1.0, 15.0, Some(Direction::East), 15.0, 1.0),
        ChestModelType::Single => (1.0, 14.0, None, 7.0, 2.0),
    };
    append_cube(
        mesh,
        sprite,
        [0.0, 19.0],
        [64.0, 64.0],
        Vec3::new(body_origin_x, 0.0, 1.0),
        Vec3::new(body_width, 10.0, 14.0),
        hidden_face,
        rotation,
        world,
        light,
    );
    append_cube(
        mesh,
        sprite,
        [0.0, 0.0],
        [64.0, 64.0],
        Vec3::new(body_origin_x, 0.0, 0.0),
        Vec3::new(body_width, 5.0, 14.0),
        hidden_face,
        animated_part,
        world,
        light,
    );
    append_cube(
        mesh,
        sprite,
        [0.0, 0.0],
        [64.0, 64.0],
        Vec3::new(lock_origin_x, -2.0, 14.0),
        Vec3::new(lock_width, 4.0, 1.0),
        hidden_face,
        animated_part,
        world,
        light,
    );
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ChestMaterial {
    EnderChest,
    Christmas,
    Trapped,
    CopperUnaffected,
    CopperExposed,
    CopperWeathered,
    CopperOxidized,
    Regular,
}
impl ChestMaterial {
    fn prefix(self) -> &'static str {
        match self {
            Self::EnderChest => "ender",
            Self::Christmas => "christmas",
            Self::Trapped => "trapped",
            Self::CopperUnaffected => "copper",
            Self::CopperExposed => "copper_exposed",
            Self::CopperWeathered => "copper_weathered",
            Self::CopperOxidized => "copper_oxidized",
            Self::Regular => "normal",
        }
    }
}
fn chest_material(path: &str, xmas_textures: bool) -> ChestMaterial {
    if path.contains("copper_chest") {
        return if path.contains("oxidized") {
            ChestMaterial::CopperOxidized
        } else if path.contains("weathered") {
            ChestMaterial::CopperWeathered
        } else if path.contains("exposed") {
            ChestMaterial::CopperExposed
        } else {
            ChestMaterial::CopperUnaffected
        };
    }
    if path.contains("ender_chest") {
        return ChestMaterial::EnderChest;
    }
    if xmas_textures {
        return ChestMaterial::Christmas;
    }
    if path.contains("trapped_chest") {
        return ChestMaterial::Trapped;
    }
    ChestMaterial::Regular
}
fn is_extended_christmas_date(month: u32, day: u32) -> bool {
    month == 12 && matches!(day, 24 | 25 | 26)
}
fn xmas_textures() -> bool {
    static XMAS_TEXTURES: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *XMAS_TEXTURES.get_or_init(|| {
        let seconds = web_time::SystemTime::now()
            .duration_since(web_time::SystemTime::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or(0);
        let (month, day) = utc_month_day_from_unix_days((seconds / 86_400) as i64);
        is_extended_christmas_date(month, day)
    })
}
fn utc_month_day_from_unix_days(unix_days: i64) -> (u32, u32) {
    let shifted = unix_days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    (month, day)
}
fn chest_texture(material: ChestMaterial, model_type: ChestModelType) -> ResourceLocation {
    let prefix = material.prefix();
    let suffix = match model_type {
        ChestModelType::Left if material != ChestMaterial::EnderChest => "_left",
        ChestModelType::Right if material != ChestMaterial::EnderChest => "_right",
        _ => "",
    };
    ResourceLocation::new("minecraft", &format!("entity/chest/{prefix}{suffix}"))
}
fn chest_rotation(facing: Direction) -> Mat4 {
    let angle = -facing.to_y_rot().to_radians();
    Mat4::from_translation(Vec3::new(0.5, 0.0, 0.5))
        * Mat4::from_rotation_y(angle)
        * Mat4::from_translation(Vec3::new(-0.5, 0.0, -0.5))
}
#[allow(clippy::too_many_arguments)]
fn append_cube(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    texture_offset: [f32; 2],
    texture_size: [f32; 2],
    origin: Vec3,
    dimensions: Vec3,
    hidden_face: Option<Direction>,
    transform: Mat4,
    world: Vec3,
    light: [f32; 2],
) {
    append_colored_cube(
        mesh,
        sprite,
        texture_offset,
        texture_size,
        origin,
        dimensions,
        hidden_face,
        transform,
        world,
        light,
        [1.0; 4],
    );
}
#[allow(clippy::too_many_arguments)]
fn append_colored_cube(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    texture_offset: [f32; 2],
    texture_size: [f32; 2],
    origin: Vec3,
    dimensions: Vec3,
    hidden_face: Option<Direction>,
    transform: Mat4,
    world: Vec3,
    light: [f32; 2],
    color: [f32; 4],
) {
    append_colored_cube_to(
        &mut mesh.vertices,
        sprite,
        texture_offset,
        texture_size,
        origin,
        dimensions,
        hidden_face,
        transform,
        world,
        light,
        color,
        &mut mesh.indices,
    );
}
#[allow(clippy::too_many_arguments)]
fn append_colored_cube_to(
    vertices: &mut Vec<Vertex>,
    sprite: SpriteUvRect,
    texture_offset: [f32; 2],
    texture_size: [f32; 2],
    origin: Vec3,
    dimensions: Vec3,
    hidden_face: Option<Direction>,
    transform: Mat4,
    world: Vec3,
    light: [f32; 2],
    color: [f32; 4],
    indices: &mut Vec<u32>,
) {
    let min = origin / 16.0;
    let max = (origin + dimensions) / 16.0;
    let p000 = Vec3::new(min.x, min.y, min.z);
    let p100 = Vec3::new(max.x, min.y, min.z);
    let p110 = Vec3::new(max.x, max.y, min.z);
    let p010 = Vec3::new(min.x, max.y, min.z);
    let p001 = Vec3::new(min.x, min.y, max.z);
    let p101 = Vec3::new(max.x, min.y, max.z);
    let p111 = Vec3::new(max.x, max.y, max.z);
    let p011 = Vec3::new(min.x, max.y, max.z);
    let width = dimensions.x;
    let height = dimensions.y;
    let depth = dimensions.z;
    let u0 = texture_offset[0];
    let u1 = u0 + depth;
    let u2 = u1 + width;
    let u22 = u2 + width;
    let u3 = u2 + depth;
    let u4 = u3 + width;
    let v0 = texture_offset[1];
    let v1 = v0 + depth;
    let v2 = v1 + height;
    let faces = [
        (Direction::Down, [p101, p001, p000, p100], [u1, v0, u2, v1]),
        (Direction::Up, [p110, p010, p011, p111], [u2, v1, u22, v0]),
        (Direction::West, [p000, p001, p011, p010], [u0, v1, u1, v2]),
        (Direction::North, [p100, p000, p010, p110], [u1, v1, u2, v2]),
        (Direction::East, [p101, p100, p110, p111], [u2, v1, u3, v2]),
        (Direction::South, [p001, p101, p111, p011], [u3, v1, u4, v2]),
    ];
    for (direction, positions, [face_u0, face_v0, face_u1, face_v1]) in faces {
        if hidden_face == Some(direction) {
            continue;
        }
        let normal = transform
            .transform_vector3(direction_vector(direction))
            .normalize_or_zero()
            .to_array();
        let uvs = [
            atlas_uv(sprite, texture_size, face_u1, face_v0),
            atlas_uv(sprite, texture_size, face_u0, face_v0),
            atlas_uv(sprite, texture_size, face_u0, face_v1),
            atlas_uv(sprite, texture_size, face_u1, face_v1),
        ];
        let start = vertices.len() as u32;
        for index in 0..4 {
            vertices.push(Vertex {
                position: (transform.transform_point3(positions[index]) + world).to_array(),
                tex_coords: uvs[index],
                normal,
                color,
                light,
            });
        }
        indices.extend_from_slice(&[start, start + 1, start + 2, start + 2, start + 3, start]);
    }
}
#[allow(clippy::too_many_arguments)]
fn append_quad(
    mesh: &mut ChunkMesh,
    sprite: SpriteUvRect,
    positions: [Vec3; 4],
    uv_pixels: [[f32; 2]; 4],
    texture_size: [f32; 2],
    direction: Direction,
    transform: Mat4,
    world: Vec3,
    light: [f32; 2],
) {
    let normal = transform
        .transform_vector3(direction_vector(direction))
        .normalize_or_zero()
        .to_array();
    let start = mesh.vertices.len() as u32;
    for index in 0..4 {
        mesh.vertices.push(Vertex {
            position: (transform.transform_point3(positions[index]) + world).to_array(),
            tex_coords: atlas_uv(
                sprite,
                texture_size,
                uv_pixels[index][0],
                uv_pixels[index][1],
            ),
            normal,
            color: [1.0, 1.0, 1.0, 1.0],
            light,
        });
    }
    mesh.indices
        .extend_from_slice(&[start, start + 1, start + 2, start + 2, start + 3, start]);
}
fn atlas_uv(sprite: SpriteUvRect, texture_size: [f32; 2], u: f32, v: f32) -> [f32; 2] {
    sprite.map_unit_uv(u / texture_size[0], v / texture_size[1])
}
fn direction_vector(direction: Direction) -> Vec3 {
    let (x, y, z) = direction.step();
    Vec3::new(x as f32, y as f32, z as f32)
}
