use crate::model_v3::*;
use crate::strict::{array, ensure, field, id, keys, resource_id, string, uint};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, String>;

fn closed_engine_ids() -> BTreeMap<&'static str, Value> {
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
    ])
}

fn is_engine(id: &str) -> bool {
    id.starts_with("engine.")
}

fn parse_resource(r: &Value) -> Result<ResourceV3> {
    let kind = string(field(r, "kind")?)?;
    match kind {
        "transient_texture" => {
            keys(
                r,
                &[
                    "id",
                    "kind",
                    "format",
                    "usage",
                    "samples",
                    "width_from",
                    "height_from",
                ],
                &[],
            )?;
            let name = resource_id(field(r, "id")?)?.to_owned();
            let format = TextureFormatV3::parse(string(field(r, "format")?)?)
                .ok_or_else(|| "unsupported v3 texture format".to_string())?;
            let usage = TextureUsageV3::parse(string(field(r, "usage")?)?)
                .ok_or_else(|| "unsupported v3 texture usage".to_string())?;
            ensure(
                format_usage_compatible(format, usage),
                "v3 format/usage mismatch",
            )?;
            ensure(uint(field(r, "samples")?)? == 1, "v3 texture samples")?;
            ensure(
                string(field(r, "width_from")?)? == "output"
                    && string(field(r, "height_from")?)? == "output",
                "v3 texture sizing",
            )?;
            Ok(ResourceV3::Texture {
                id: name,
                format,
                usage,
                samples: 1,
            })
        }
        "transient_buffer" => {
            keys(r, &["id", "kind", "size_bytes", "usage"], &[])?;
            let name = resource_id(field(r, "id")?)?.to_owned();
            let size = uint(field(r, "size_bytes")?)?;
            ensure(
                (16..=16_777_216).contains(&size) && size % 16 == 0,
                "v3 buffer size_bytes",
            )?;
            let usage = TextureUsageV3::parse(string(field(r, "usage")?)?)
                .ok_or_else(|| "unsupported v3 buffer usage".to_string())?;
            ensure(
                usage == TextureUsageV3::Storage,
                "v3 buffers are storage-only",
            )?;
            Ok(ResourceV3::Buffer {
                id: name,
                size_bytes: size,
                usage,
            })
        }
        "sampler" => {
            keys(
                r,
                &[
                    "id",
                    "kind",
                    "mag_filter",
                    "min_filter",
                    "mipmap_filter",
                    "address_u",
                    "address_v",
                    "address_w",
                ],
                &[],
            )?;
            let name = resource_id(field(r, "id")?)?.to_owned();
            ensure(
                string(field(r, "mag_filter")?)? == "linear"
                    && string(field(r, "min_filter")?)? == "linear",
                "v3 sampler filter",
            )?;
            ensure(
                string(field(r, "mipmap_filter")?)? == "nearest",
                "v3 sampler mipmap",
            )?;
            for key in ["address_u", "address_v", "address_w"] {
                ensure(
                    string(field(r, key)?)? == "clamp-to-edge",
                    "v3 sampler address",
                )?;
            }
            Ok(ResourceV3::Sampler { id: name })
        }
        _ => Err("unsupported v3 resource kind".into()),
    }
}

fn resource_usage(resource: &ResourceV3) -> Option<TextureUsageV3> {
    match resource {
        ResourceV3::Texture { usage, .. } | ResourceV3::Buffer { usage, .. } => Some(*usage),
        ResourceV3::Sampler { .. } => None,
    }
}

pub fn validate_graph_v3(v: &Value) -> Result<ValidatedGraphV3> {
    validate_graph_with_inputs(v, &closed_engine_ids())
}

pub(crate) fn validate_graph_with_inputs(
    v: &Value,
    closed: &BTreeMap<&str, Value>,
) -> Result<ValidatedGraphV3> {
    keys(
        v,
        &[
            "graph_version",
            "engine_inputs",
            "resources",
            "passes",
            "final_outputs",
            "limits",
        ],
        &[],
    )?;
    ensure(
        uint(field(v, "graph_version")?)? == GRAPH_V3_VERSION,
        "unsupported v3 graph version",
    )?;

    let engine_inputs = array(field(v, "engine_inputs")?)?;
    ensure(engine_inputs.len() <= closed.len(), "v3 engine input count")?;
    let mut seen_engine = BTreeSet::new();
    for input in engine_inputs {
        keys(
            input,
            &["id", "kind", "semantic"],
            &["format", "samples", "size"],
        )?;
        let input_id = string(field(input, "id")?)?;
        let expected = closed
            .get(input_id)
            .ok_or_else(|| format!("closed v3 engine input: {input_id}"))?;
        ensure(input == expected, "v3 engine input contract")?;
        ensure(
            seen_engine.insert(input_id.to_owned()),
            "duplicate v3 engine input",
        )?;
    }

    let limits = field(v, "limits")?;
    keys(
        limits,
        &[
            "max_texture_dimension_2d",
            "max_color_attachments",
            "samples",
        ],
        &[],
    )?;
    let min_dimension = uint(field(limits, "max_texture_dimension_2d")?)?;
    ensure(
        (512..=8192).contains(&min_dimension)
            && uint(field(limits, "max_color_attachments")?)? == 1
            && uint(field(limits, "samples")?)? == 1,
        "v3 declared limits",
    )?;

    let resources = array(field(v, "resources")?)?;
    ensure(
        (1..=MAX_V3_RESOURCES).contains(&resources.len()),
        "v3 resource count",
    )?;
    let mut by_id: BTreeMap<String, ResourceV3> = BTreeMap::new();
    for r in resources {
        let resource = parse_resource(r)?;
        ensure(
            by_id.insert(resource.id().to_owned(), resource).is_none(),
            "duplicate v3 resource",
        )?;
    }

    let passes = array(field(v, "passes")?)?;
    ensure((1..=MAX_V3_PASSES).contains(&passes.len()), "v3 pass count")?;
    let mut parsed: Vec<PassV3> = Vec::new();
    let mut pass_ids = BTreeSet::new();
    let mut producers: BTreeMap<String, String> = BTreeMap::new();
    let mut used: BTreeSet<String> = BTreeSet::new();
    for p in passes {
        keys(p, &["id", "stage", "inputs", "samplers", "outputs"], &[])?;
        let name = id(field(p, "id")?)?.to_owned();
        ensure(pass_ids.insert(name.clone()), "duplicate v3 pass")?;
        let stage = StageV3::parse(string(field(p, "stage")?)?)
            .ok_or_else(|| "unsupported v3 stage".to_string())?;
        let inputs = array(field(p, "inputs")?)?;
        ensure(
            inputs.len() <= MAX_V3_INPUTS_PER_PASS.max(closed.len()),
            "v3 input count",
        )?;
        let samplers = array(field(p, "samplers")?)?;
        ensure(
            samplers.len() <= MAX_V3_SAMPLERS_PER_PASS,
            "v3 sampler count",
        )?;
        let outputs = array(field(p, "outputs")?)?;
        ensure(
            (1..=MAX_V3_OUTPUTS_PER_PASS).contains(&outputs.len()),
            "v3 output count",
        )?;

        let mut in_ids: Vec<String> = Vec::new();
        let mut in_set = BTreeSet::new();
        for item in inputs {
            let dep = string(item)?.to_owned();
            ensure(in_set.insert(dep.clone()), "duplicate v3 input")?;
            if is_engine(&dep) {
                ensure(
                    seen_engine.contains(&dep),
                    &format!("undeclared or closed v3 engine input: {dep}"),
                )?;
            } else {
                let target = by_id
                    .get(&dep)
                    .ok_or_else(|| format!("unknown v3 input: {dep}"))?;
                ensure(!target.is_sampler(), "v3 sampler is not a texture input")?;
                if target.is_buffer() {
                    ensure(
                        stage == StageV3::Compute,
                        "v3 buffers are compute-only inputs",
                    )?;
                } else if resource_usage(target) == Some(TextureUsageV3::Storage) {
                    ensure(
                        stage == StageV3::Compute,
                        "v3 storage textures are compute-only inputs",
                    )?;
                }
            }
            used.insert(dep.clone());
            in_ids.push(dep);
        }
        let mut sampler_ids: Vec<String> = Vec::new();
        let mut sampler_set = BTreeSet::new();
        for item in samplers {
            let dep = string(item)?.to_owned();
            ensure(sampler_set.insert(dep.clone()), "duplicate v3 sampler")?;
            let target = by_id
                .get(&dep)
                .ok_or_else(|| format!("unknown v3 sampler: {dep}"))?;
            ensure(target.is_sampler(), "v3 sampler reference")?;
            used.insert(dep.clone());
            sampler_ids.push(dep);
        }
        let mut out_ids: Vec<String> = Vec::new();
        let mut out_set = BTreeSet::new();
        for item in outputs {
            let dest = string(item)?.to_owned();
            ensure(out_set.insert(dest.clone()), "duplicate v3 output")?;
            ensure(!is_engine(&dest), "v3 engine inputs are read-only")?;
            let target = by_id
                .get(&dest)
                .ok_or_else(|| format!("unknown v3 output: {dest}"))?;
            ensure(!target.is_sampler(), "v3 sampler is not writable")?;
            ensure(!in_set.contains(&dest), "v3 read/write alias in one pass")?;
            if target.is_buffer() {
                ensure(
                    stage == StageV3::Compute,
                    "v3 buffers are compute-only outputs",
                )?;
            } else if let Some(usage) = resource_usage(target) {
                match stage {
                    StageV3::Shadow => ensure(
                        usage == TextureUsageV3::DepthTarget,
                        "v3 shadow writes depth only",
                    )?,
                    StageV3::Geometry => ensure(
                        usage == TextureUsageV3::ColorTarget
                            || usage == TextureUsageV3::DepthTarget,
                        "v3 geometry output usage",
                    )?,
                    StageV3::Lighting | StageV3::Post => ensure(
                        usage == TextureUsageV3::ColorTarget,
                        "v3 lighting/post write color targets only",
                    )?,
                    StageV3::Compute => ensure(
                        usage == TextureUsageV3::Storage,
                        "v3 compute writes storage only",
                    )?,
                }
            }
            ensure(
                producers.insert(dest.clone(), name.clone()).is_none(),
                "v3 multiple resource producers",
            )?;
            used.insert(dest.clone());
            out_ids.push(dest);
        }
        let color_targets = out_ids
            .iter()
            .filter(|dest| {
                by_id.get(*dest).and_then(resource_usage) == Some(TextureUsageV3::ColorTarget)
            })
            .count();
        let depth_targets = out_ids
            .iter()
            .filter(|dest| {
                by_id.get(*dest).and_then(resource_usage) == Some(TextureUsageV3::DepthTarget)
            })
            .count();
        ensure(color_targets <= 1, "v3 color attachment limit per pass")?;
        ensure(depth_targets <= 1, "v3 depth attachment limit per pass")?;
        parsed.push(PassV3 {
            id: name,
            stage,
            inputs: in_ids,
            samplers: sampler_ids,
            outputs: out_ids,
        });
    }

    for (name, resource) in &by_id {
        if resource.is_sampler() {
            ensure(used.contains(name), "v3 unused sampler")?;
        } else {
            ensure(
                used.contains(name) && producers.contains_key(name),
                "v3 unused/uninitialized resource",
            )?;
        }
    }

    let finals = array(field(v, "final_outputs")?)?;
    ensure(
        (1..=MAX_V3_FINAL_OUTPUTS).contains(&finals.len()),
        "v3 final output count",
    )?;
    let mut final_ids: Vec<String> = Vec::new();
    let mut final_set = BTreeSet::new();
    for item in finals {
        keys(item, &["resource"], &[])?;
        let dest = string(field(item, "resource")?)?.to_owned();
        ensure(final_set.insert(dest.clone()), "duplicate v3 final output")?;
        let target = by_id
            .get(&dest)
            .ok_or_else(|| format!("unknown v3 final output: {dest}"))?;
        ensure(
            target.is_texture(),
            "v3 final outputs are textures (buffer readback closed)",
        )?;
        ensure(producers.contains_key(&dest), "v3 unproduced final output")?;
        final_ids.push(dest);
    }

    let order = topo_order(&parsed, &producers)?;
    let ordered: Vec<PassV3> = order
        .iter()
        .map(|name| {
            parsed
                .iter()
                .find(|p| &p.id == name)
                .cloned()
                .ok_or_else(|| "v3 validated pass id".to_string())
        })
        .collect::<Result<Vec<_>>>()?;
    let lifetimes = lifetimes(&ordered, &by_id, &final_ids)?;

    Ok(ValidatedGraphV3 {
        version: GRAPH_V3_VERSION,
        min_texture_dimension: min_dimension as u32,
        passes: ordered,
        resources: by_id,
        final_outputs: final_ids,
        lifetimes,
    })
}

fn topo_order(passes: &[PassV3], producers: &BTreeMap<String, String>) -> Result<Vec<String>> {
    let mut pending: BTreeSet<String> = passes.iter().map(|p| p.id.clone()).collect();
    let by_id: BTreeMap<&str, &PassV3> = passes.iter().map(|p| (p.id.as_str(), p)).collect();
    let mut done: BTreeSet<String> = BTreeSet::new();
    let mut order: Vec<String> = Vec::new();
    while !pending.is_empty() {
        let next = pending
            .iter()
            .find(|name| {
                let pass = by_id.get(name.as_str());
                match pass {
                    Some(pass) => pass.inputs.iter().all(|input| {
                        if is_engine(input) {
                            true
                        } else {
                            producers
                                .get(input)
                                .is_some_and(|producer| done.contains(producer))
                        }
                    }),
                    None => false,
                }
            })
            .cloned()
            .ok_or_else(|| "v3 cyclic or uninitialized graph".to_string())?;
        pending.remove(&next);
        done.insert(next.clone());
        order.push(next);
    }
    Ok(order)
}

fn lifetimes(
    passes: &[PassV3],
    resources: &BTreeMap<String, ResourceV3>,
    finals: &[String],
) -> Result<BTreeMap<String, ResourceLifetimeV3>> {
    let mut out: BTreeMap<String, ResourceLifetimeV3> = BTreeMap::new();
    for (step, pass) in passes.iter().enumerate() {
        for input in &pass.inputs {
            if is_engine(input) {
                continue;
            }
            let slot = out
                .get_mut(input)
                .ok_or_else(|| format!("v3 pass {} reads {input} before its producer", pass.id))?;
            slot.last_use_step = step;
        }
        for dest in &pass.outputs {
            ensure(
                resources.contains_key(dest),
                "v3 writes an undeclared resource",
            )?;
            ensure(
                out.insert(
                    dest.clone(),
                    ResourceLifetimeV3 {
                        first_write_step: step,
                        last_use_step: step,
                    },
                )
                .is_none(),
                &format!("v3 texture {dest} has multiple producers"),
            )?;
        }
    }
    for (name, resource) in resources {
        if !resource.is_sampler() {
            ensure(
                out.contains_key(name),
                "v3 declared resource has no producer",
            )?;
        }
    }
    for dest in finals {
        let slot = out
            .get_mut(dest)
            .ok_or_else(|| "v3 final output has no producer".to_string())?;
        slot.last_use_step = passes.len();
    }
    Ok(out)
}
