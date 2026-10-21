use crate::graph_v3::validate_graph_v3;
use crate::model_v3::{ResourceV3, StageV3, ValidatedGraphV3};
use crate::strict::{array, ensure, field, id, keys, string, uint};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, String>;

pub const DESCRIPTOR_V4_VERSION: u64 = 4;
pub const SHADER_API_V4: &str = "1.3";
pub const PACK_PROFILE_V4: &str = "graph_v3_executable_v1";
pub const PACK_PROFILE_FRAME: &str = "graph_v3_frame_v1";
pub const PACK_PROFILE_TEMPORAL: &str = "graph_v3_temporal_v1";
pub const PACK_PROFILE_LIGHTING: &str = "graph_v3_lighting_v1";
pub const HISTORY_BINDINGS: [(u32, &str); 2] =
    [(10, "engine.history_color"), (11, "engine.history_camera")];

#[derive(Clone, Debug)]
pub struct HistoryConfig {
    pub source: String,
    pub format: crate::model_v3::TextureFormatV3,
}

#[derive(Clone, Debug)]
pub struct LightingConfig {
    pub output: String,
}

fn history_engine_inputs(
    format: crate::model_v3::TextureFormatV3,
) -> BTreeMap<&'static str, Value> {
    BTreeMap::from([
        (
            "engine.history_color",
            serde_json::json!({"id":"engine.history_color","kind":"engine_texture","format":format.as_str(),"samples":1,"semantic":"previous_submitted_color_v1"}),
        ),
        (
            "engine.history_camera",
            serde_json::json!({"id":"engine.history_camera","kind":"engine_uniform","size":192,"semantic":"previous_submitted_camera_v1"}),
        ),
    ])
}

pub const FRAME_BINDINGS: [(u32, &str); 5] = [
    (3, "engine.scene_depth"),
    (4, "engine.camera"),
    (5, "engine.environment"),
    (6, "engine.opaque_normal"),
    (7, "engine.opaque_albedo"),
];
pub const SHADOW_BINDINGS: [(u32, &str); 2] =
    [(8, "engine.terrain_shadow"), (9, "engine.shadow_camera")];
pub const LIGHT_BINDING: (u32, &str) = (12, "engine.opaque_light");
pub const PREFIX_FRAME_BINDINGS: [(u32, &str); 5] = [
    (3, "engine.opaque_depth"),
    (4, "engine.camera"),
    (5, "engine.environment"),
    (6, "engine.opaque_normal"),
    (7, "engine.opaque_albedo"),
];

pub fn light_engine_input() -> Value {
    serde_json::json!({"id":"engine.opaque_light","kind":"engine_texture","format":"rg16float","samples":1,"semantic":"opaque_effective_block_sky_uv2_256_v1"})
}

fn shadow_engine_inputs() -> BTreeMap<&'static str, Value> {
    let mut inputs = frame_engine_inputs();
    inputs.insert("engine.terrain_shadow",serde_json::json!({"id":"engine.terrain_shadow","kind":"engine_texture","format":"depth32float","samples":1,"semantic":"terrain_sun_shadow_forward_z_v1"}));
    inputs.insert("engine.shadow_camera",serde_json::json!({"id":"engine.shadow_camera","kind":"engine_uniform","semantic":"terrain_sun_shadow_camera_v1","size":96}));
    inputs
}

pub fn frame_engine_inputs() -> BTreeMap<&'static str, Value> {
    BTreeMap::from([
        (
            "engine.scene_color",
            serde_json::json!({"id":"engine.scene_color","kind":"engine_texture","format":"rgba8unorm","samples":1,"semantic":"scene_color_v1"}),
        ),
        (
            "engine.scene_depth",
            serde_json::json!({"id":"engine.scene_depth","kind":"engine_texture","format":"depth32float","samples":1,"semantic":"world_depth_reverse_z_v1"}),
        ),
        (
            "engine.camera",
            serde_json::json!({"id":"engine.camera","kind":"engine_uniform","semantic":"world_camera_v1","size":176}),
        ),
        (
            "engine.environment",
            serde_json::json!({"id":"engine.environment","kind":"engine_uniform","semantic":"world_environment_v1","size":80}),
        ),
        (
            "engine.opaque_normal",
            serde_json::json!({"id":"engine.opaque_normal","kind":"engine_texture","format":"rgba16float","samples":1,"semantic":"opaque_world_normal_encoded_v1"}),
        ),
        (
            "engine.opaque_albedo",
            serde_json::json!({"id":"engine.opaque_albedo","kind":"engine_texture","format":"rgba8unorm","samples":1,"semantic":"opaque_tinted_albedo_v1"}),
        ),
    ])
}

pub fn prefix_engine_inputs() -> BTreeMap<&'static str, Value> {
    let mut inputs = frame_engine_inputs();
    inputs.remove("engine.scene_color");
    inputs.remove("engine.scene_depth");
    inputs.insert("engine.world_color", serde_json::json!({"id":"engine.world_color","kind":"engine_texture","format":"rgba8unorm","samples":1,"semantic":"opaque_terrain_prefix_color_v1"}));
    inputs.insert("engine.opaque_depth", serde_json::json!({"id":"engine.opaque_depth","kind":"engine_texture","format":"depth32float","samples":1,"semantic":"opaque_terrain_prefix_depth_reverse_z_v1"}));
    inputs.get_mut("engine.opaque_normal").unwrap()["semantic"] =
        "terrain_prefix_world_normal_encoded_v1".into();
    inputs.get_mut("engine.opaque_albedo").unwrap()["semantic"] =
        "terrain_prefix_tinted_albedo_v1".into();
    inputs.insert(LIGHT_BINDING.1, light_engine_input());
    inputs
}

pub fn entity_engine_inputs(shadow: bool) -> BTreeMap<&'static str, Value> {
    let mut inputs = if shadow {
        shadow_engine_inputs()
    } else {
        frame_engine_inputs()
    };
    inputs.get_mut("engine.opaque_normal").unwrap()["semantic"] =
        "terrain_cutout_entity_world_normal_encoded_v1".into();
    inputs.get_mut("engine.opaque_albedo").unwrap()["semantic"] =
        "terrain_cutout_entity_tinted_albedo_v1".into();
    if shadow {
        inputs.get_mut("engine.terrain_shadow").unwrap()["semantic"] =
            "terrain_cutout_entity_sun_shadow_forward_z_v1".into();
    }
    inputs
}

fn executable_stage(stage: StageV3, pass_id: &str, lighting_profile: bool) -> Result<()> {
    match stage {
        StageV3::Post => Ok(()),
        StageV3::Compute if !lighting_profile => Ok(()),
        StageV3::Compute => Err("lighting profile admits lighting/post stages only".into()),
        StageV3::Lighting if lighting_profile => Ok(()),
        StageV3::Geometry => Err(format!(
            "v4 pass '{pass_id}': stage 'geometry' has no GPU hook yet \
             (authored world-visibility execution is unavailable); only 'post'/'compute' execute"
        )),
        StageV3::Shadow => Err(format!(
            "v4 pass '{pass_id}': stage 'shadow' has no GPU hook yet \
             (authored shadow-caster execution is unavailable); only 'post'/'compute' execute"
        )),
        StageV3::Lighting => Err(format!(
            "v4 pass '{pass_id}': stage 'lighting' has no GPU hook yet \
             (authored lighting-stage execution is unavailable); only 'post'/'compute' execute"
        )),
    }
}

fn entry_syntax(entry: &str) -> Result<()> {
    ensure(
        (1..=64).contains(&entry.len())
            && entry.bytes().enumerate().all(|(i, b)| {
                b.is_ascii_alphabetic() || b == b'_' || (i > 0 && b.is_ascii_digit())
            }),
        "v4 entry syntax",
    )
}

#[derive(Clone, Debug)]
pub enum ProgramV4 {
    Lighting {
        source: String,
        vertex_entry: String,
        fragment_entry: String,
        has_params: bool,
    },
    Post {
        source: String,
        vertex_entry: String,
        fragment_entry: String,
        frame_inputs: bool,
        entities: bool,
        has_params: bool,
        shadow: bool,
        history: bool,
        light: bool,
    },
    Compute {
        source: String,
        compute_entry: String,
        storage_bytes: u64,
        workgroup_size: [u32; 3],
    },
}

impl ProgramV4 {
    pub fn stage(&self) -> StageV3 {
        match self {
            ProgramV4::Lighting { .. } => StageV3::Lighting,
            ProgramV4::Post { .. } => StageV3::Post,
            ProgramV4::Compute { .. } => StageV3::Compute,
        }
    }

    pub fn storage_bytes(&self) -> Option<u64> {
        match self {
            ProgramV4::Compute { storage_bytes, .. } => Some(*storage_bytes),
            ProgramV4::Post { .. } | ProgramV4::Lighting { .. } => None,
        }
    }

    pub fn workgroup_size(&self) -> Option<[u32; 3]> {
        match self {
            ProgramV4::Compute { workgroup_size, .. } => Some(*workgroup_size),
            ProgramV4::Post { .. } | ProgramV4::Lighting { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BoundPassV4 {
    pub id: String,
    pub stage: StageV3,
    pub program: String,
    pub input: Option<String>,
    pub sampler: Option<String>,
    pub output: String,
    pub dispatch: Option<[u32; 3]>,
}

#[derive(Clone, Debug)]
pub struct ValidatedPackV4 {
    pub version: u64,
    pub profile: String,
    pub min_texture_dimension: u32,
    pub programs: BTreeMap<String, ProgramV4>,
    pub graph: ValidatedGraphV3,
    pub passes: Vec<BoundPassV4>,
    pub params: Option<Vec<u8>>,
    pub settings: Value,
    param_definitions: Value,
    pub shadow: Option<crate::shadow_v4::ShadowConfig>,
    pub history: Option<HistoryConfig>,
    pub lighting: Option<LightingConfig>,
}

impl ValidatedPackV4 {
    pub fn param_definitions(&self) -> &Value {
        &self.param_definitions
    }

    pub fn apply_settings(&mut self, settings: &Value) -> Result<()> {
        let object = settings
            .as_object()
            .ok_or("v4 settings must be an object")?;
        ensure(
            matches!(
                self.profile.as_str(),
                PACK_PROFILE_FRAME | PACK_PROFILE_TEMPORAL | PACK_PROFILE_LIGHTING
            ) || object.is_empty(),
            "v4 shaderpack takes no settings",
        )?;
        let ids = crate::params::ids(&self.param_definitions)?;
        ensure(
            object.keys().all(|id| ids.contains(id)),
            "unknown v4 setting",
        )?;
        let bytes = crate::params::parse(&self.param_definitions, settings)?;
        let mut resolved = serde_json::Map::new();
        for definition in array(&self.param_definitions)? {
            let id = string(field(definition, "id")?)?;
            resolved.insert(
                id.to_owned(),
                object
                    .get(id)
                    .unwrap_or(field(definition, "default")?)
                    .clone(),
            );
        }
        self.params = bytes;
        self.settings = Value::Object(resolved);
        Ok(())
    }
    pub fn needs_frame_inputs(&self) -> bool {
        self.programs.values().any(|p| {
            matches!(
                p,
                ProgramV4::Post {
                    frame_inputs: true,
                    ..
                } | ProgramV4::Lighting { .. }
            )
        })
    }
    pub fn needs_entity_inputs(&self) -> bool {
        self.programs
            .values()
            .any(|p| matches!(p, ProgramV4::Post { entities: true, .. }))
    }
    pub fn needs_light_inputs(&self) -> bool {
        self.programs.values().any(|p| {
            matches!(
                p,
                ProgramV4::Post { light: true, .. } | ProgramV4::Lighting { .. }
            )
        })
    }
    pub fn logical_bytes(&self, width: u64, height: u64) -> Option<u64> {
        self.graph.logical_bytes(width, height)
    }

    pub fn needs_compute(&self) -> bool {
        self.passes.iter().any(|p| p.stage == StageV3::Compute)
    }
}

pub(crate) fn validate_compute_wgsl(source: &str, entry: &str) -> Result<(u64, [u32; 3])> {
    ensure(
        source.len() <= 1_048_576 && !source.starts_with('\u{feff}'),
        "v4 compute WGSL byte bound/BOM",
    )?;
    let module = naga::front::wgsl::parse_str(source).map_err(|e| e.emit_to_string(source))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| e.to_string())?;
    ensure(
        module
            .constants
            .iter()
            .all(|(_, c)| c.r#override == naga::Override::None),
        "v4 compute override constants are closed",
    )?;
    ensure(
        module.entry_points.len() == 1,
        "v4 compute expects exactly one entry point",
    )?;
    let point = &module.entry_points[0];
    ensure(
        point.stage == naga::ShaderStage::Compute,
        "v4 compute entry must be a compute shader",
    )?;
    ensure(point.name == entry, "v4 missing compute entry")?;
    ensure(
        point.workgroup_size.iter().all(|&n| n > 0),
        "v4 compute workgroup size",
    )?;
    let workgroup_size = point.workgroup_size;
    let mut storage_bytes = None;
    for (_, global) in module.global_variables.iter() {
        let binding = global.binding.as_ref().ok_or("v4 compute unbound global")?;
        ensure(
            binding.group == 0 && binding.binding == 0,
            "v4 compute bind group/layout",
        )?;
        match global.space {
            naga::AddressSpace::Storage { access } => {
                ensure(
                    access.contains(naga::StorageAccess::LOAD | naga::StorageAccess::STORE),
                    "v4 compute storage must be read_write",
                )?;
            }
            _ => return Err("v4 compute binding 0 must be read_write storage".into()),
        }
        match &module.types[global.ty].inner {
            naga::TypeInner::Array { base, size, stride } => {
                ensure(
                    module.types[*base].inner
                        == naga::TypeInner::Scalar(naga::Scalar {
                            kind: naga::ScalarKind::Uint,
                            width: 4,
                        }),
                    "v4 compute storage element",
                )?;
                let count = match size {
                    naga::ArraySize::Constant(n) => u64::from(n.get()),
                    naga::ArraySize::Dynamic => {
                        return Err("v4 compute storage array must be sized".into());
                    }
                };
                let bytes = (*stride as u64)
                    .checked_mul(count)
                    .ok_or("v4 compute storage size overflow")?;
                ensure(
                    storage_bytes.replace(bytes).is_none(),
                    "v4 compute expects one storage global",
                )?;
            }
            _ => return Err("v4 compute binding 0 must be a u32 array".into()),
        }
    }
    storage_bytes
        .ok_or_else(|| "v4 compute declares no storage".to_string())
        .map(|bytes| (bytes, workgroup_size))
}

fn parse_program(
    entry: &Value,
    sources: &BTreeMap<String, String>,
    frame_profile: bool,
    temporal_profile: bool,
    lighting_profile: bool,
) -> Result<(String, ProgramV4)> {
    let stage = StageV3::parse(string(field(entry, "stage")?)?)
        .ok_or_else(|| "unsupported v4 program stage".to_string())?;
    match stage {
        StageV3::Lighting if lighting_profile => {
            keys(
                entry,
                &[
                    "id",
                    "stage",
                    "vertex_entry",
                    "fragment_entry",
                    "frame_inputs",
                    "lighting_inputs",
                ],
                &["path"],
            )?;
            ensure(
                string(field(entry, "frame_inputs")?)? == "terrain_prefix_v1",
                "lighting requires terrain_prefix_v1 frame inputs",
            )?;
            ensure(
                string(field(entry, "lighting_inputs")?)? == "block_sky_uv2_v1",
                "lighting requires block_sky_uv2_v1 inputs",
            )?;
            let name = id(field(entry, "id")?)?.to_owned();
            let vertex = string(field(entry, "vertex_entry")?)?;
            let fragment = string(field(entry, "fragment_entry")?)?;
            entry_syntax(vertex)?;
            entry_syntax(fragment)?;
            let source = sources
                .get(&name)
                .ok_or_else(|| format!("v4 unknown program source: {name}"))?;
            let has_params = crate::validation::validate_frame_wgsl(
                source, vertex, fragment, false, false, true,
            )
            .map_err(|e| format!("v4 lighting program '{name}': {e}"))?;
            Ok((
                name,
                ProgramV4::Lighting {
                    source: source.clone(),
                    vertex_entry: vertex.to_owned(),
                    fragment_entry: fragment.to_owned(),
                    has_params,
                },
            ))
        }
        StageV3::Post => {
            keys(
                entry,
                &["id", "stage", "vertex_entry", "fragment_entry"],
                &["path", "frame_inputs", "history_inputs", "lighting_inputs"],
            )?;
            ensure(
                !lighting_profile
                    || ["frame_inputs", "history_inputs", "lighting_inputs"]
                        .iter()
                        .all(|key| entry.get(key).is_none()),
                "lighting profile late post programs use color/settings only",
            )?;
            let name = id(field(entry, "id")?)?.to_owned();
            let vertex = string(field(entry, "vertex_entry")?)?;
            let fragment = string(field(entry, "fragment_entry")?)?;
            entry_syntax(vertex)?;
            entry_syntax(fragment)?;
            let source = sources
                .get(&name)
                .ok_or_else(|| format!("v4 unknown program source: {name}"))?;
            let frame_inputs = match entry.get("frame_inputs") {
                None => false,
                Some(Value::String(value)) if frame_profile && value == "world_opaque_v1" => true,
                Some(Value::String(value)) if frame_profile && value == "world_shadow_v1" => true,
                Some(Value::String(value))
                    if frame_profile
                        && matches!(
                            value.as_str(),
                            "world_opaque_entities_v1" | "world_shadow_entities_v1"
                        ) =>
                {
                    true
                }
                _ => return Err("frame_inputs requires graph_v3_frame_v1 / world_opaque_v1".into()),
            };
            let mode = entry.get("frame_inputs").and_then(Value::as_str);
            let shadow = matches!(mode, Some("world_shadow_v1" | "world_shadow_entities_v1"));
            let entities = matches!(
                mode,
                Some("world_opaque_entities_v1" | "world_shadow_entities_v1")
            );
            let light = match entry.get("lighting_inputs") {
                None => false,
                Some(Value::String(value)) if frame_inputs && value == "block_sky_uv2_v1" => true,
                _ => return Err("lighting_inputs requires frame inputs / block_sky_uv2_v1".into()),
            };
            let history =
                match entry.get("history_inputs") {
                    None => false,
                    Some(Value::String(value))
                        if temporal_profile && frame_inputs && value == "previous_color_v1" =>
                    {
                        true
                    }
                    _ => return Err(
                        "history_inputs requires temporal profile/frame inputs/previous_color_v1"
                            .into(),
                    ),
                };
            let has_params = if frame_inputs {
                crate::validation::validate_frame_wgsl(
                    source, vertex, fragment, shadow, history, light,
                )
            } else {
                crate::validation::validate_wgsl(
                    source,
                    vertex,
                    fragment,
                    crate::model::Profile::Color,
                )
            }
            .map_err(|e| format!("v4 post program '{name}': {e}"))?;
            ensure(
                !has_params || frame_profile,
                "v4 params require the frame profile",
            )?;
            Ok((
                name,
                ProgramV4::Post {
                    source: source.clone(),
                    vertex_entry: vertex.to_owned(),
                    fragment_entry: fragment.to_owned(),
                    frame_inputs,
                    entities,
                    has_params,
                    shadow,
                    history,
                    light,
                },
            ))
        }
        StageV3::Compute => {
            ensure(
                !lighting_profile,
                "lighting profile admits lighting/post programs only",
            )?;
            keys(entry, &["id", "stage", "compute_entry"], &["path"])?;
            let name = id(field(entry, "id")?)?.to_owned();
            let compute = string(field(entry, "compute_entry")?)?;
            entry_syntax(compute)?;
            let source = sources
                .get(&name)
                .ok_or_else(|| format!("v4 unknown program source: {name}"))?;
            let storage_bytes = validate_compute_wgsl(source, compute)
                .map_err(|e| format!("v4 compute program '{name}': {e}"))?;
            Ok((
                name,
                ProgramV4::Compute {
                    source: source.clone(),
                    compute_entry: compute.to_owned(),
                    storage_bytes: storage_bytes.0,
                    workgroup_size: storage_bytes.1,
                },
            ))
        }
        StageV3::Geometry | StageV3::Shadow | StageV3::Lighting => Err(format!(
            "v4 program stage '{}' has no GPU hook yet; only 'post'/'compute' programs execute",
            stage.as_str(),
        )),
    }
}

pub fn admit_descriptor_v4(
    descriptor: &Value,
    sources: &BTreeMap<String, String>,
) -> Result<ValidatedPackV4> {
    keys(
        descriptor,
        &[
            "descriptor_version",
            "pack_profile",
            "programs",
            "engine_inputs",
            "resources",
            "passes",
            "final_outputs",
            "limits",
        ],
        &["params", "shadow", "history", "lighting"],
    )?;
    ensure(
        uint(field(descriptor, "descriptor_version")?)? == DESCRIPTOR_V4_VERSION,
        "unsupported v4 descriptor version",
    )?;
    let profile = string(field(descriptor, "pack_profile")?)?;
    let temporal_profile = profile == PACK_PROFILE_TEMPORAL;
    let lighting_profile = profile == PACK_PROFILE_LIGHTING;
    let frame_profile = profile == PACK_PROFILE_FRAME || temporal_profile || lighting_profile;
    ensure(
        profile == PACK_PROFILE_V4 || frame_profile,
        "unsupported v4 pack profile",
    )?;
    ensure(
        frame_profile || descriptor.get("params").is_none(),
        "v4 params require the frame profile",
    )?;
    ensure(
        lighting_profile == descriptor.get("lighting").is_some(),
        "lighting configuration requires the lighting profile and is mandatory there",
    )?;
    ensure(
        !lighting_profile
            || (descriptor.get("shadow").is_none() && descriptor.get("history").is_none()),
        "lighting profile does not admit shadow/history configuration",
    )?;
    let lighting = if let Some(value) = descriptor.get("lighting") {
        keys(value, &["output"], &[])?;
        Some(LightingConfig {
            output: string(field(value, "output")?)?.to_owned(),
        })
    } else {
        None
    };
    let definitions = descriptor
        .get("params")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));
    let params = crate::params::parse(&definitions, &serde_json::json!({}))?;
    let shadow = if let Some(value) = descriptor.get("shadow") {
        ensure(frame_profile, "shadow requires frame profile")?;
        keys(value, &["resolution", "distance", "depth_bias"], &[])?;
        let config = crate::shadow_v4::ShadowConfig {
            resolution: u32::try_from(uint(field(value, "resolution")?)?)
                .map_err(|_| "shadow resolution overflow")?,
            distance: field(value, "distance")?
                .as_f64()
                .ok_or("shadow distance number")? as f32,
            depth_bias: field(value, "depth_bias")?
                .as_f64()
                .ok_or("shadow bias number")? as f32,
        };
        config.validate()?;
        Some(config)
    } else {
        None
    };

    let programs = array(field(descriptor, "programs")?)?;
    ensure((1..=64).contains(&programs.len()), "v4 program count")?;
    let mut by_name: BTreeMap<String, ProgramV4> = BTreeMap::new();
    for entry in programs {
        let (name, program) = parse_program(
            entry,
            sources,
            frame_profile,
            temporal_profile,
            lighting_profile,
        )?;
        ensure(
            by_name.insert(name.clone(), program).is_none(),
            "duplicate v4 program",
        )?;
    }
    ensure(
        by_name.values().any(|program| {
            matches!(
                program,
                ProgramV4::Post {
                    has_params: true,
                    ..
                } | ProgramV4::Lighting {
                    has_params: true,
                    ..
                }
            )
        }) == params.is_some(),
        "v4 parameter definitions/uniform declarations mismatch",
    )?;
    ensure(
        by_name
            .values()
            .any(|program| matches!(program, ProgramV4::Post { shadow: true, .. }))
            == shadow.is_some(),
        "shadow configuration/program mismatch",
    )?;
    let entities = by_name
        .values()
        .any(|p| matches!(p, ProgramV4::Post { entities: true, .. }));
    ensure(
        !entities
            || !by_name.values().any(|p| {
                matches!(
                    p,
                    ProgramV4::Post {
                        frame_inputs: true,
                        entities: false,
                        ..
                    }
                )
            }),
        "cannot mix terrain-only and entity frame coverage",
    )?;

    let resources = array(field(descriptor, "resources")?)?;
    let mut declared: BTreeMap<String, &Value> = BTreeMap::new();
    for resource in resources {
        let kind = string(field(resource, "kind")?)?;
        let name = string(field(resource, "id")?)?;
        ensure(
            name != "pack.settings",
            "pack.settings is reserved for immutable settings",
        )?;
        ensure(
            declared.insert(name.to_owned(), resource).is_none(),
            "duplicate v4 resource",
        )?;
        match kind {
            "transient_texture" => {
                let format = string(field(resource, "format")?)?;
                let usage = string(field(resource, "usage")?)?;
                ensure(
                    (format == "rgba8unorm" || (frame_profile && format == "rgba16float"))
                        && usage == "color_target",
                    "v4 slice executes rgba8unorm color targets only",
                )?;
            }
            "transient_buffer" => {}
            "sampler" => {}
            _ => return Err("unsupported v4 resource kind".into()),
        }
    }

    let history = if let Some(value) = descriptor.get("history") {
        ensure(temporal_profile, "history requires temporal profile")?;
        keys(value, &["source"], &[])?;
        let source = string(field(value, "source")?)?.to_owned();
        let resource = declared
            .get(&source)
            .ok_or("history source must be a declared texture")?;
        ensure(
            resource["kind"] == "transient_texture",
            "history source must be a texture",
        )?;
        let format = crate::model_v3::TextureFormatV3::parse(string(field(resource, "format")?)?)
            .ok_or("history format")?;
        Some(HistoryConfig { source, format })
    } else {
        None
    };
    ensure(
        temporal_profile == history.is_some(),
        "temporal profile requires history",
    )?;
    ensure(
        by_name
            .values()
            .any(|p| matches!(p, ProgramV4::Post { history: true, .. }))
            == history.is_some(),
        "history configuration/program mismatch",
    )?;
    let mut engine_contract = if lighting_profile {
        let mut inputs = prefix_engine_inputs();
        inputs.insert(
            "engine.scene_color",
            frame_engine_inputs().remove("engine.scene_color").unwrap(),
        );
        inputs
    } else if entities {
        entity_engine_inputs(shadow.is_some())
    } else if shadow.is_some() {
        shadow_engine_inputs()
    } else {
        frame_engine_inputs()
    };
    if let Some(history) = &history {
        engine_contract.extend(history_engine_inputs(history.format));
    }
    if by_name
        .values()
        .any(|p| matches!(p, ProgramV4::Post { light: true, .. }))
    {
        engine_contract.insert(LIGHT_BINDING.1, light_engine_input());
    }
    for input in array(field(descriptor, "engine_inputs")?)? {
        let name = string(field(input, "id")?)?;
        ensure(
            name == "engine.scene_color" || (frame_profile && engine_contract.contains_key(name)),
            "v4 engine input is outside the program's declared frame contract",
        )?;
    }

    let passes = array(field(descriptor, "passes")?)?;
    let mut stripped: Vec<Value> = Vec::with_capacity(passes.len());
    let mut bound: Vec<BoundPassV4> = Vec::with_capacity(passes.len());
    for pass in passes {
        keys(
            pass,
            &[
                "id", "stage", "inputs", "samplers", "outputs", "program", "bindings",
            ],
            &["dispatch"],
        )?;
        let pass_id = id(field(pass, "id")?)?.to_owned();
        let stage = StageV3::parse(string(field(pass, "stage")?)?)
            .ok_or_else(|| "unsupported v4 stage".to_string())?;
        executable_stage(stage, &pass_id, lighting_profile)?;
        let program_name = string(field(pass, "program")?)?.to_owned();
        let program = by_name
            .get(&program_name)
            .ok_or_else(|| format!("v4 unknown program: {program_name}"))?;
        ensure(program.stage() == stage, "v4 program/pass stage mismatch")?;
        let inputs = array(field(pass, "inputs")?)?;
        let samplers = array(field(pass, "samplers")?)?;
        let outputs = array(field(pass, "outputs")?)?;
        let bindings = field(pass, "bindings")?;
        let bindings_object = bindings
            .as_object()
            .ok_or("v4 bindings must be an object")?;
        match stage {
            StageV3::Lighting => {
                let has_params = matches!(
                    program,
                    ProgramV4::Lighting {
                        has_params: true,
                        ..
                    }
                );
                ensure(
                    inputs.len() == 7,
                    "lighting pass takes color and six prefix inputs",
                )?;
                ensure(samplers.len() == 1, "lighting pass takes one sampler")?;
                ensure(outputs.len() == 1, "lighting pass writes one output")?;
                ensure(
                    pass.get("dispatch").is_none(),
                    "lighting pass takes no dispatch",
                )?;
                let input = string(&inputs[0])?.to_owned();
                let sampler = string(&samplers[0])?.to_owned();
                let output = string(&outputs[0])?.to_owned();
                ensure(
                    bindings_object.len() == 8 + usize::from(has_params)
                        && bindings_object.get("0/0").and_then(Value::as_str)
                            == Some(input.as_str())
                        && bindings_object.get("0/1").and_then(Value::as_str)
                            == Some(sampler.as_str()),
                    "lighting bindings must map 0/0 to color and 0/1 to the sampler",
                )?;
                if has_params {
                    ensure(
                        bindings_object.get("0/2").and_then(Value::as_str) == Some("pack.settings"),
                        "lighting binding 0/2 must be pack.settings",
                    )?;
                }
                for (index, (binding, name)) in PREFIX_FRAME_BINDINGS.iter().enumerate() {
                    ensure(
                        inputs[index + 1].as_str() == Some(name)
                            && bindings_object
                                .get(&format!("0/{binding}"))
                                .and_then(Value::as_str)
                                == Some(name),
                        "lighting prefix inputs/bindings mismatch",
                    )?;
                }
                ensure(
                    inputs[6].as_str() == Some(LIGHT_BINDING.1)
                        && bindings_object.get("0/12").and_then(Value::as_str)
                            == Some(LIGHT_BINDING.1),
                    "lighting input/binding must match block_sky_uv2_v1",
                )?;
                if input != "engine.world_color" {
                    match declared.get(&input) {
                        Some(resource) => ensure(
                            string(field(resource, "kind")?)? == "transient_texture",
                            "lighting reads world color or lighting-produced textures only",
                        )?,
                        None => return Err(format!("v4 unknown lighting color input: {input}")),
                    }
                }
                bound.push(BoundPassV4 {
                    id: pass_id.clone(),
                    stage,
                    program: program_name,
                    input: Some(input),
                    sampler: Some(sampler),
                    output,
                    dispatch: None,
                });
            }
            StageV3::Post => {
                let frame_inputs = matches!(
                    program,
                    ProgramV4::Post {
                        frame_inputs: true,
                        ..
                    }
                );
                let has_params = matches!(
                    program,
                    ProgramV4::Post {
                        has_params: true,
                        ..
                    }
                );
                let wants_shadow = matches!(program, ProgramV4::Post { shadow: true, .. });
                let wants_history = matches!(program, ProgramV4::Post { history: true, .. });
                let wants_light = matches!(program, ProgramV4::Post { light: true, .. });
                ensure(
                    inputs.len()
                        == (if frame_inputs { 6 } else { 1 })
                            + if wants_shadow { 2 } else { 0 }
                            + if wants_history { 2 } else { 0 }
                            + usize::from(wants_light),
                    "v4 post input count does not match program",
                )?;
                ensure(samplers.len() == 1, "v4 post pass takes one sampler")?;
                ensure(outputs.len() == 1, "v4 post pass writes one output")?;
                ensure(pass.get("dispatch").is_none(), "v4 post takes no dispatch")?;
                let input = string(&inputs[0])?.to_owned();
                let sampler = string(&samplers[0])?.to_owned();
                let output = string(&outputs[0])?.to_owned();
                ensure(
                    bindings_object.len()
                        == (if frame_inputs { 7 } else { 2 })
                            + usize::from(has_params)
                            + if wants_shadow { 2 } else { 0 }
                            + if wants_history { 2 } else { 0 }
                            + usize::from(wants_light)
                        && bindings_object.get("0/0").and_then(Value::as_str)
                            == Some(input.as_str())
                        && bindings_object.get("0/1").and_then(Value::as_str)
                            == Some(sampler.as_str()),
                    "v4 post bindings must map 0/0 to the input and 0/1 to the sampler",
                )?;
                if has_params {
                    ensure(
                        bindings_object.get("0/2").and_then(Value::as_str) == Some("pack.settings"),
                        "v4 binding 0/2 must be pack.settings",
                    )?;
                }
                if frame_inputs {
                    for (index, (binding, name)) in FRAME_BINDINGS.iter().enumerate() {
                        ensure(
                            inputs[index + 1].as_str() == Some(name)
                                && bindings_object
                                    .get(&format!("0/{binding}"))
                                    .and_then(Value::as_str)
                                    == Some(name),
                            "frame inputs/bindings must match world_opaque_v1",
                        )?;
                    }
                }
                if wants_shadow {
                    for (index, (binding, name)) in SHADOW_BINDINGS.iter().enumerate() {
                        ensure(
                            inputs[6 + index].as_str() == Some(name)
                                && bindings_object
                                    .get(&format!("0/{binding}"))
                                    .and_then(Value::as_str)
                                    == Some(name),
                            "shadow inputs/bindings mismatch",
                        )?;
                    }
                }
                if wants_history {
                    for (index, (binding, name)) in HISTORY_BINDINGS.iter().enumerate() {
                        ensure(
                            inputs[index + 6 + if wants_shadow { 2 } else { 0 }].as_str()
                                == Some(name)
                                && bindings_object
                                    .get(&format!("0/{binding}"))
                                    .and_then(Value::as_str)
                                    == Some(name),
                            "history inputs/bindings must match previous_color_v1",
                        )?;
                    }
                }
                if wants_light {
                    let (binding, name) = LIGHT_BINDING;
                    let index =
                        6 + if wants_shadow { 2 } else { 0 } + if wants_history { 2 } else { 0 };
                    ensure(
                        inputs[index].as_str() == Some(name)
                            && bindings_object
                                .get(&format!("0/{binding}"))
                                .and_then(Value::as_str)
                                == Some(name),
                        "lighting input/binding must match block_sky_uv2_v1",
                    )?;
                }
                if input != "engine.scene_color" {
                    match declared.get(&input) {
                        Some(resource) => ensure(
                            string(field(resource, "kind")?)? == "transient_texture",
                            "v4 post reads textures only",
                        )?,
                        None => return Err(format!("v4 unknown post input: {input}")),
                    }
                }
                bound.push(BoundPassV4 {
                    id: pass_id.clone(),
                    stage,
                    program: program_name,
                    input: Some(input),
                    sampler: Some(sampler),
                    output,
                    dispatch: None,
                });
            }
            StageV3::Compute => {
                ensure(
                    inputs.is_empty(),
                    "v4 compute passes take no texture inputs in this slice",
                )?;
                ensure(samplers.is_empty(), "v4 compute takes no samplers")?;
                ensure(outputs.len() == 1, "v4 compute writes one buffer")?;
                let output = string(&outputs[0])?.to_owned();
                ensure(
                    bindings_object.len() == 1
                        && bindings_object.get("0/0").and_then(Value::as_str)
                            == Some(output.as_str()),
                    "v4 compute bindings must map 0/0 to the written buffer",
                )?;
                let dispatch = field(pass, "dispatch")?;
                let items = array(dispatch)?;
                ensure(items.len() == 3, "v4 compute dispatch is [x, y, z]")?;
                let mut groups = [0u32; 3];
                for (i, item) in items.iter().enumerate() {
                    let n = uint(item)?;
                    ensure((1..=65535).contains(&n), "v4 compute dispatch range")?;
                    groups[i] = n as u32;
                }
                let buffer = declared
                    .get(&output)
                    .ok_or_else(|| format!("v4 unknown compute output: {output}"))?;
                ensure(
                    string(field(buffer, "kind")?)? == "transient_buffer",
                    "v4 compute writes storage buffers only",
                )?;
                let size = uint(field(buffer, "size_bytes")?)?;
                let storage_bytes = match program {
                    ProgramV4::Compute { storage_bytes, .. } => *storage_bytes,
                    ProgramV4::Post { .. } | ProgramV4::Lighting { .. } => {
                        return Err("v4 program/pass stage mismatch".into());
                    }
                };
                ensure(
                    size == storage_bytes,
                    "v4 compute buffer size must match the shader storage array",
                )?;
                bound.push(BoundPassV4 {
                    id: pass_id.clone(),
                    stage,
                    program: program_name,
                    input: None,
                    sampler: None,
                    output,
                    dispatch: Some(groups),
                });
            }
            StageV3::Geometry | StageV3::Shadow => {
                return Err(format!(
                    "v4 pass '{pass_id}': stage '{}' has no GPU hook yet",
                    stage.as_str()
                ));
            }
        }
        stripped.push(serde_json::json!({
            "id": pass["id"],
            "stage": pass["stage"],
            "inputs": pass["inputs"],
            "samplers": pass["samplers"],
            "outputs": pass["outputs"],
        }));
    }

    let graph_value = serde_json::json!({
        "graph_version": 3,
        "engine_inputs": descriptor["engine_inputs"],
        "resources": descriptor["resources"],
        "passes": stripped,
        "final_outputs": descriptor["final_outputs"],
        "limits": descriptor["limits"],
    });
    let mut graph = if frame_profile {
        crate::graph_v3::validate_graph_with_inputs(&graph_value, &engine_contract)
    } else {
        validate_graph_v3(&graph_value)
    }
    .map_err(|e| format!("v4 graph: {e}"))?;

    let by_id: BTreeMap<&str, BoundPassV4> =
        bound.iter().map(|p| (p.id.as_str(), p.clone())).collect();
    let mut ordered = Vec::with_capacity(by_id.len());
    for pass in &graph.passes {
        ordered.push(
            by_id
                .get(pass.id.as_str())
                .cloned()
                .ok_or_else(|| "v4 validated pass id".to_string())?,
        );
    }
    if let Some(lighting) = &lighting {
        validate_lighting_phases(&mut graph, &mut ordered, lighting)?;
    }
    for name in &graph.final_outputs {
        ensure(
            matches!(
                graph.resources.get(name),
                Some(ResourceV3::Texture {
                    format: crate::model_v3::TextureFormatV3::Rgba8Unorm,
                    ..
                })
            ),
            "v4 final output must be rgba8unorm; add a tone-mapping pass for HDR",
        )?;
        let producer = ordered
            .iter()
            .find(|p| &p.output == name)
            .ok_or_else(|| format!("v4 final output has no bound producer: {name}"))?;
        ensure(
            producer.stage == StageV3::Post,
            "v4 final outputs must be post-pass color targets",
        )?;
    }
    for pass in &ordered {
        match graph.resources.get(&pass.output) {
            Some(ResourceV3::Texture { .. }) => ensure(
                pass.stage == StageV3::Post
                    || (lighting_profile && pass.stage == StageV3::Lighting),
                "v4 texture outputs come from admitted fullscreen passes",
            )?,
            Some(ResourceV3::Buffer { .. }) => ensure(
                pass.stage == StageV3::Compute,
                "v4 buffer outputs come from compute passes",
            )?,
            Some(ResourceV3::Sampler { .. }) | None => {
                return Err(format!("v4 pass '{}' writes no texture/buffer", pass.id));
            }
        }
    }

    if let Some(history) = &history {
        ensure(
            ordered
                .iter()
                .any(|p| p.output == history.source && p.stage == StageV3::Post),
            "history source must have an executed post producer",
        )?;
    }
    let mut pack = ValidatedPackV4 {
        version: DESCRIPTOR_V4_VERSION,
        profile: profile.to_owned(),
        min_texture_dimension: graph.min_texture_dimension,
        programs: by_name,
        graph,
        passes: ordered,
        params,
        settings: serde_json::json!({}),
        param_definitions: definitions,
        shadow,
        history,
        lighting,
    };
    pack.apply_settings(&serde_json::json!({}))?;
    Ok(pack)
}

fn validate_lighting_phases(
    graph: &mut ValidatedGraphV3,
    ordered: &mut Vec<BoundPassV4>,
    lighting: &LightingConfig,
) -> Result<()> {
    ensure(
        matches!(
            graph.resources.get(&lighting.output),
            Some(ResourceV3::Texture {
                format: crate::model_v3::TextureFormatV3::Rgba8Unorm,
                ..
            })
        ),
        "lighting output must be a declared rgba8unorm color target; tone HDR before composition",
    )?;
    let producers: BTreeMap<&str, StageV3> = ordered
        .iter()
        .map(|pass| (pass.output.as_str(), pass.stage))
        .collect();
    ensure(
        producers.get(lighting.output.as_str()) == Some(&StageV3::Lighting),
        "lighting output must have an executed lighting producer",
    )?;
    for pass in &graph.passes {
        for input in pass
            .inputs
            .iter()
            .filter(|input| !input.starts_with("engine."))
        {
            ensure(
                producers.get(input.as_str()) == Some(&pass.stage),
                "lighting/post transient reads cannot cross the host composition barrier",
            )?;
        }
    }
    ordered.sort_by_key(|pass| usize::from(pass.stage != StageV3::Lighting));
    graph
        .passes
        .sort_by_key(|pass| usize::from(pass.stage != StageV3::Lighting));
    let lighting_end = ordered
        .iter()
        .take_while(|pass| pass.stage == StageV3::Lighting)
        .count();
    let mut lifetimes = BTreeMap::<String, crate::model_v3::ResourceLifetimeV3>::new();
    for (step, pass) in graph.passes.iter().enumerate() {
        for input in pass
            .inputs
            .iter()
            .filter(|input| !input.starts_with("engine."))
        {
            let lifetime = lifetimes
                .get_mut(input)
                .ok_or("lighting phase reads before its producer")?;
            lifetime.last_use_step = step;
        }
        for output in &pass.outputs {
            ensure(
                lifetimes
                    .insert(
                        output.clone(),
                        crate::model_v3::ResourceLifetimeV3 {
                            first_write_step: step,
                            last_use_step: step,
                        },
                    )
                    .is_none(),
                "lighting phase has multiple resource producers",
            )?;
        }
    }
    let boundary = lifetimes
        .get_mut(&lighting.output)
        .ok_or("lighting output lifetime missing")?;
    boundary.last_use_step = boundary.last_use_step.max(lighting_end);
    for final_output in &graph.final_outputs {
        lifetimes
            .get_mut(final_output)
            .ok_or("lighting final lifetime missing")?
            .last_use_step = graph.passes.len();
    }
    graph.lifetimes = lifetimes;
    Ok(())
}

pub fn load_v4(
    snapshot: &dyn el_platform_api::ResourceSnapshot,
    receipt: &[u8],
) -> Result<ValidatedPackV4> {
    load_v4_with_settings(snapshot, receipt, &serde_json::json!({}))
}

pub fn load_v4_with_settings(
    snapshot: &dyn el_platform_api::ResourceSnapshot,
    receipt: &[u8],
    settings: &Value,
) -> Result<ValidatedPackV4> {
    use el_extension_api::{ExtensionId, PackagePath, PackageVersion, Sha256Digest};
    use el_platform_api::VirtualPath;
    use sha2::{Digest, Sha256};

    fn read(
        snapshot: &dyn el_platform_api::ResourceSnapshot,
        path: &str,
        limit: usize,
    ) -> Result<Vec<u8>> {
        let vpath = VirtualPath::new(path).map_err(|e| e.to_string())?;
        let bytes = snapshot
            .read_view(&vpath)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("missing {vpath}"))?;
        ensure(bytes.as_ref().len() <= limit, "v4 snapshot file byte limit")?;
        Ok(bytes.as_ref().to_vec())
    }

    let bytes = crate::package::read_manifest(snapshot)?;
    let manifest = crate::strict::strict_json(&bytes)?;
    keys(
        &manifest,
        &[
            "manifest_version",
            "kind",
            "id",
            "version",
            "display_name",
            "license",
            "files",
            "shader_api",
            "shader",
        ],
        &["description"],
    )?;
    ensure(
        uint(field(&manifest, "manifest_version")?)? == 1,
        "v4 manifest version",
    )?;
    ensure(
        string(field(&manifest, "kind")?)? == "shaderpack",
        "v4 kind",
    )?;
    ensure(
        string(field(&manifest, "shader_api")?)? == SHADER_API_V4,
        "v4 shader API",
    )?;
    let pack_id: ExtensionId = crate::strict::typed(field(&manifest, "id")?)?;
    ensure(
        !pack_id.is_reserved_namespace(),
        "v4 reserved package namespace",
    )?;
    let version: PackageVersion = crate::strict::typed(field(&manifest, "version")?)?;
    crate::strict::text_limit(field(&manifest, "display_name")?, 1, 64)?;
    if let Some(description) = manifest.get("description") {
        crate::strict::text_limit(description, 0, 512)?;
    }
    let license = string(field(&manifest, "license")?)?;
    ensure(
        (1..=128).contains(&license.len())
            && license
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".+-".contains(&b)),
        "v4 license syntax",
    )?;
    let receipt_value = crate::strict::strict_json(receipt)?;
    keys(&receipt_value, &["id", "version", "content_id"], &[])?;
    let mut hasher = Sha256::new();
    hasher.update(b"rustore.package.v1\0");
    hasher.update(&bytes);
    let content_id = Sha256Digest::from_bytes(hasher.finalize().into());
    ensure(
        crate::strict::typed::<ExtensionId>(field(&receipt_value, "id")?)? == pack_id
            && crate::strict::typed::<PackageVersion>(field(&receipt_value, "version")?)?
                == version
            && crate::strict::typed::<Sha256Digest>(field(&receipt_value, "content_id")?)?
                == content_id,
        "v4 receipt mismatch",
    )?;
    let files = array(field(&manifest, "files")?)?;
    ensure((1..=4096).contains(&files.len()), "v4 file count")?;
    let mut payload: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    let mut total = 0u64;
    for file in files {
        keys(file, &["path", "size", "sha256"], &[])?;
        let path: PackagePath = crate::strict::typed(field(file, "path")?)?;
        ensure(
            path.as_str().starts_with("shaders/"),
            "v4 payload must be under shaders/",
        )?;
        let limit = if path.as_str().ends_with(".wgsl") {
            1_048_576
        } else if path.as_str().ends_with(".json") {
            262_144
        } else {
            return Err("unsupported v4 payload extension".into());
        };
        let size = uint(field(file, "size")?)?;
        ensure(size <= limit as u64, "v4 declared file size")?;
        total = total.checked_add(size).ok_or("v4 payload size overflow")?;
        ensure(total <= 32 * 1024 * 1024, "v4 payload budget")?;
        let data = read(snapshot, path.as_str(), limit)?;
        let expected: Sha256Digest = crate::strict::typed(field(file, "sha256")?)?;
        ensure(
            data.len() as u64 == size
                && Sha256Digest::from_bytes(Sha256::digest(&data).into()) == expected,
            "v4 payload size/hash",
        )?;
        ensure(
            payload.insert(path.to_string(), data).is_none(),
            "v4 duplicate payload path",
        )?;
    }
    let actual = snapshot
        .list(&VirtualPath::new("shaders").map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let actual_set: BTreeSet<_> = actual.iter().map(|p| p.as_str()).collect();
    ensure(
        actual_set.len() == actual.len()
            && actual_set == payload.keys().map(String::as_str).collect(),
        "v4 missing/unlisted payload",
    )?;
    let shader = field(&manifest, "shader")?;
    keys(shader, &["descriptor"], &[])?;
    let descriptor_path: PackagePath = crate::strict::typed(field(shader, "descriptor")?)?;
    ensure(
        descriptor_path.as_str().ends_with(".json"),
        "v4 descriptor extension",
    )?;
    let descriptor_bytes = payload
        .get(descriptor_path.as_str())
        .ok_or("v4 unlisted descriptor")?;
    let descriptor = crate::strict::strict_json(descriptor_bytes)?;
    let mut sources: BTreeMap<String, String> = BTreeMap::new();
    for program in array(field(&descriptor, "programs")?)? {
        keys(
            program,
            &["id"],
            &[
                "stage",
                "vertex_entry",
                "fragment_entry",
                "compute_entry",
                "path",
                "frame_inputs",
                "history_inputs",
                "lighting_inputs",
            ],
        )?;
        let name = id(field(program, "id")?)?.to_owned();
        let path_value = program.get("path");
        let candidate = path_value
            .map(|v| string(v).map(str::to_owned))
            .transpose()?
            .unwrap_or_else(|| format!("shaders/{name}.wgsl"));
        let raw = payload
            .get(&candidate)
            .ok_or_else(|| format!("v4 unlisted program: {name}"))?;
        let text = std::str::from_utf8(raw).map_err(|e| format!("v4 program utf8: {e}"))?;
        ensure(
            sources.insert(name, text.to_owned()).is_none(),
            "duplicate v4 program",
        )?;
    }
    let mut pack = admit_descriptor_v4(&descriptor, &sources)?;
    pack.apply_settings(settings)?;
    Ok(pack)
}
