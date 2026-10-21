use crate::{model::*, strict::*};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, String>;
pub(crate) fn validate_graph(
    v: &Value,
    programs: &BTreeMap<String, Program>,
    profile: Profile,
    settings: &Value,
) -> Result<Variant> {
    keys(
        v,
        &[
            "id",
            "capabilities_required",
            "limits_required",
            "params",
            "resources",
            "passes",
            "final_output",
        ],
        &[],
    )?;
    let caps = array(field(v, "capabilities_required")?)?;
    let mut cs = BTreeSet::new();
    for c in caps {
        id(c)?;
        ensure(cs.insert(string(c)?), "duplicate shader capability")?;
    }
    ensure(
        {
            let base: &[&str] = if profile == Profile::Color {
                &["post_color_v1", "filterable_rgba8", "sampler_linear"]
            } else if profile.has_environment() {
                &[
                    "post_color_depth_environment_v1",
                    "filterable_rgba8",
                    "sampler_linear",
                    "depth32float",
                    "camera_uniform_v1",
                    "environment_uniform_v1",
                ]
            } else {
                &[
                    "post_color_depth_v1",
                    "filterable_rgba8",
                    "sampler_linear",
                    "depth32float",
                    "camera_uniform_v1",
                ]
            };
            base.iter().all(|c| cs.contains(c))
        },
        "unsupported fixture capabilities",
    )?;
    let limits = field(v, "limits_required")?;
    keys(
        limits,
        &[
            "max_texture_dimension_2d",
            "max_color_attachments",
            "samples",
        ],
        &[],
    )?;
    ensure(
        (512..=8192).contains(&uint(field(limits, "max_texture_dimension_2d")?)?)
            && uint(field(limits, "max_color_attachments")?)? == 1
            && uint(field(limits, "samples")?)? == 1,
        "declared limits",
    )?;
    let params = crate::params::parse(field(v, "params")?, settings)?;
    ensure(
        cs.contains("params_uniform_v1") == params.is_some(),
        "params capability mismatch",
    )?;
    let resources = array(field(v, "resources")?)?;
    ensure((1..=64).contains(&resources.len()), "resource count")?;
    let mut types = BTreeMap::new();
    for r in resources {
        let name = resource_id(field(r, "id")?)?;
        let kind = string(field(r, "kind")?)?;
        ensure(types.insert(name, kind).is_none(), "duplicate resource")?;
        match kind {
            "transient_texture" => {
                keys(
                    r,
                    &[
                        "id",
                        "kind",
                        "format",
                        "samples",
                        "width_from",
                        "height_from",
                    ],
                    &[],
                )?;
                eq(field(r, "format")?, "rgba8unorm")?;
                ensure(uint(field(r, "samples")?)? == 1, "texture samples")?;
                eq(field(r, "width_from")?, "output")?;
                eq(field(r, "height_from")?, "output")?;
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
                for k in ["mag_filter", "min_filter"] {
                    eq(field(r, k)?, "linear")?;
                }
                eq(field(r, "mipmap_filter")?, "nearest")?;
                for k in ["address_u", "address_v", "address_w"] {
                    eq(field(r, k)?, "clamp-to-edge")?;
                }
            }
            _ => return Err("unsupported resource kind".into()),
        }
    }
    let passes = array(field(v, "passes")?)?;
    ensure((1..=32).contains(&passes.len()), "pass count")?;
    let mut pass_ids = BTreeSet::new();
    let mut producers = BTreeMap::new();
    let mut edges = BTreeMap::new();
    let mut used = BTreeSet::new();
    for p in passes {
        keys(
            p,
            &[
                "id",
                "stage",
                "program",
                "bindings",
                "color_load",
                "color_store",
                "outputs",
            ],
            &["enabled"],
        )?;
        let name = id(field(p, "id")?)?;
        ensure(pass_ids.insert(name), "duplicate pass")?;
        eq(field(p, "stage")?, "post")?;
        ensure(
            programs.contains_key(string(field(p, "program")?)?),
            "unknown program",
        )?;
        if let Some(enabled) = p.get("enabled") {
            ensure(
                enabled.as_bool() == Some(true),
                "disabled passes outside v1",
            )?;
        }
        let b = field(p, "bindings")?;
        if profile.has_depth() {
            keys(b, &["0/0", "0/1", "0/3"], &[])?;
            eq(field(b, "0/3")?, "engine.scene_depth")?;
        } else {
            keys(b, &["0/0", "0/1"], &[])?;
        }
        ensure(
            programs[string(field(p, "program")?)?].has_params == params.is_some(),
            "program params layout mismatch",
        )?;
        let input = string(field(b, "0/0")?)?;
        let sampler = string(field(b, "0/1")?)?;
        ensure(
            input == "engine.scene_color" || types.get(input) == Some(&"transient_texture"),
            "texture input",
        )?;
        ensure(types.get(sampler) == Some(&"sampler"), "sampler input")?;
        used.insert(input);
        used.insert(sampler);
        eq(field(p, "color_load")?, "clear")?;
        eq(field(p, "color_store")?, "store")?;
        let outputs = array(field(p, "outputs")?)?;
        ensure(outputs.len() == 1, "one output attachment")?;
        let output = &outputs[0];
        keys(output, &["location", "resource", "format"], &[])?;
        ensure(
            uint(field(output, "location")?)? == 0,
            "attachment location",
        )?;
        eq(field(output, "format")?, "rgba8unorm")?;
        let dest = string(field(output, "resource")?)?;
        ensure(
            types.get(dest) == Some(&"transient_texture") && dest != input,
            "output type/read-write alias",
        )?;
        ensure(
            producers.insert(dest, name).is_none(),
            "multiple resource producers",
        )?;
        used.insert(dest);
        edges.insert(name, input);
    }
    for (name, kind) in &types {
        ensure(
            used.contains(name) && (*kind == "sampler" || producers.contains_key(name)),
            "unused/uninitialized resource",
        )?;
    }
    let final_output = field(v, "final_output")?;
    keys(final_output, &["resource"], &[])?;
    ensure(
        producers.contains_key(string(field(final_output, "resource")?)?),
        "unproduced final output",
    )?;
    let mut pending = pass_ids;
    let mut done = BTreeSet::new();
    let mut order = Vec::new();
    while !pending.is_empty() {
        let next = pending
            .iter()
            .copied()
            .find(|n| {
                let input = edges[n];
                input == "engine.scene_color"
                    || producers.get(input).is_some_and(|p| done.contains(p))
            })
            .ok_or("cyclic or uninitialized graph")?;
        pending.remove(next);
        done.insert(next);
        order.push(next);
    }
    let sorted_passes = order
        .iter()
        .map(|name| {
            let p = passes
                .iter()
                .find(|p| p["id"].as_str() == Some(*name))
                .expect("validated pass id");
            Ok(Pass {
                id: (*name).into(),
                program: string(field(p, "program")?)?.into(),
                input: string(field(field(p, "bindings")?, "0/0")?)?.into(),
                sampler: string(field(field(p, "bindings")?, "0/1")?)?.into(),
                output: string(field(&array(field(p, "outputs")?)?[0], "resource")?)?.into(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let texture_names: Vec<String> = types
        .iter()
        .filter(|(_, kind)| **kind == "transient_texture")
        .map(|(name, _)| (*name).into())
        .collect();
    let final_name = string(field(final_output, "resource")?)?;
    let texture_lifetimes = texture_lifetimes(&sorted_passes, &texture_names, final_name)?;
    Ok(Variant {
        id: id(field(v, "id")?)?.into(),
        capabilities: cs.into_iter().map(str::to_owned).collect(),
        min_texture_dimension: uint(field(limits, "max_texture_dimension_2d")?)? as u32,
        params,
        textures: texture_names,
        samplers: types
            .iter()
            .filter(|(_, k)| **k == "sampler")
            .map(|(n, _)| (*n).into())
            .collect(),
        passes: sorted_passes,
        final_output: final_name.into(),
        texture_lifetimes,
    })
}

fn texture_lifetimes(
    passes: &[Pass],
    textures: &[String],
    final_output: &str,
) -> Result<BTreeMap<String, TextureLifetime>> {
    let declared: BTreeSet<&str> = textures.iter().map(String::as_str).collect();
    let mut lifetimes: BTreeMap<String, TextureLifetime> = BTreeMap::new();
    for (step, pass) in passes.iter().enumerate() {
        if pass.input != "engine.scene_color" {
            let input = lifetimes.get_mut(&pass.input).ok_or_else(|| {
                format!("pass {} reads {} before its producer", pass.id, pass.input)
            })?;
            input.last_use_step = step;
        }
        ensure(
            declared.contains(pass.output.as_str()),
            &format!("pass {} writes an undeclared texture", pass.id),
        )?;
        ensure(
            lifetimes
                .insert(
                    pass.output.clone(),
                    TextureLifetime {
                        first_write_step: step,
                        last_use_step: step,
                    },
                )
                .is_none(),
            &format!("texture {} has multiple producers", pass.output),
        )?;
    }
    ensure(
        lifetimes.len() == declared.len(),
        "declared texture has no producer",
    )?;
    let output = lifetimes
        .get_mut(final_output)
        .ok_or("final output has no producer")?;
    output.last_use_step = passes.len();
    Ok(lifetimes)
}
