use crate::graph_v4::ValidatedPackV4;
use crate::model_v3::{ResourceV3, StageV3};
use crate::strict::ensure;
use std::collections::BTreeMap;

type Result<T> = std::result::Result<T, String>;

fn gpu_format(format: crate::model_v3::TextureFormatV3) -> wgpu::TextureFormat {
    match format {
        crate::model_v3::TextureFormatV3::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        crate::model_v3::TextureFormatV3::Rgba16Float => wgpu::TextureFormat::Rgba16Float,
        crate::model_v3::TextureFormatV3::Depth32Float => wgpu::TextureFormat::Depth32Float,
    }
}

#[derive(Clone, Debug)]
pub struct V4Readback {
    pub width: u32,
    pub height: u32,
    pub final_rgba: Vec<u8>,
    pub buffers: BTreeMap<String, Vec<u8>>,
}

struct Target {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
}

pub struct RuntimeV4 {
    profile: String,
    min_texture_dimension: u32,
    passes: Vec<crate::graph_v4::BoundPassV4>,
    programs: BTreeMap<String, crate::graph_v4::ProgramV4>,
    resources: BTreeMap<String, ResourceV3>,
    final_output: String,
    textures: BTreeMap<String, Target>,
    buffers: BTreeMap<String, wgpu::Buffer>,
    buffer_sizes: BTreeMap<String, u64>,
    samplers: BTreeMap<String, wgpu::Sampler>,
    post_pipes: BTreeMap<String, wgpu::RenderPipeline>,
    compute_pipes: BTreeMap<String, wgpu::ComputePipeline>,
    post_layout: wgpu::BindGroupLayout,
    frame: Option<crate::frame_v4::FrameGpu>,
    params: Option<wgpu::Buffer>,
    shadow: Option<crate::shadow_v4::ShadowGpu>,
    shadow_config: Option<crate::shadow_v4::ShadowConfig>,
    entities: bool,
    light: bool,
    lighting: Option<crate::lighting_v4::LightingGpu>,
    lighting_phase: LightingPhase,
    history: Option<crate::history_v4::HistoryGpu>,
    compute_layout: Option<wgpu::BindGroupLayout>,
    size: [u32; 2],
    logical: u64,
    bytes: u64,
    budget: u64,
    frames_run: u64,
    generation: u64,
    scene_format: wgpu::TextureFormat,
    device_limits: V4DeviceLimitsReport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LightingPhase {
    Idle,
    Captured,
    Composed,
}

fn nonzero(value: u64) -> Option<std::num::NonZeroU64> {
    std::num::NonZeroU64::new(value)
}

pub fn padded_row_bytes(width: u32) -> Option<u64> {
    let row = u64::from(width).checked_mul(4)?;
    row.div_ceil(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT))
        .checked_mul(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT))
}

pub fn padded_final_staging_bytes(width: u32, height: u32) -> Option<u64> {
    padded_row_bytes(width)?.checked_mul(u64::from(height))
}

pub fn storage_staging_bytes(pack: &ValidatedPackV4) -> Option<u64> {
    let mut total: u64 = 0;
    for resource in pack.graph.resources.values() {
        if let ResourceV3::Buffer { size_bytes, .. } = resource {
            total = total.checked_add(*size_bytes)?;
        }
    }
    Some(total)
}

pub fn estimate_peak_bytes(pack: &ValidatedPackV4, width: u32, height: u32) -> Option<u64> {
    let logical = pack.logical_bytes(u64::from(width), u64::from(height))?;
    let padded = padded_final_staging_bytes(width, height)?;
    let staging = storage_staging_bytes(pack)?;
    logical
        .checked_add(padded)?
        .checked_add(staging)?
        .checked_add(if pack.params.is_some() { 1024 } else { 0 })?
        .checked_add(pack.shadow.map_or(0, |config| config.bytes()))?
        .checked_add(if pack.needs_entity_inputs() {
            crate::entities_v4::RESERVATION
        } else {
            0
        })?
        .checked_add(match &pack.history {
            Some(h) => crate::history_v4::reservation(h, [width, height])?,
            None => 0,
        })?
        .checked_add(if pack.needs_frame_inputs() {
            crate::frame_v4::reservation([width, height])?
        } else {
            0
        })?
        .checked_add(if pack.needs_light_inputs() {
            crate::frame_v4::light_reservation([width, height])?
        } else {
            0
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct V4ComputeLimitEntry {
    pub pass: String,
    pub program: String,
    pub resource: String,
    pub workgroup_size: [u32; 3],
    pub invocations_per_workgroup: u64,
    pub dispatch: [u32; 3],
    pub total_invocations: u64,
    pub storage_bytes: u64,
    pub buffer_bytes: u64,
}

#[derive(Clone, Debug)]
pub struct V4DeviceLimitsReport {
    pub effective: wgpu::Limits,
    pub entries: Vec<V4ComputeLimitEntry>,
}

impl V4DeviceLimitsReport {
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "compute_passes": self.entries.iter().map(|e| serde_json::json!({
                "pass": e.pass,
                "program": e.program,
                "resource": e.resource,
                "workgroup_size": e.workgroup_size,
                "invocations_per_workgroup": e.invocations_per_workgroup,
                "dispatch": e.dispatch,
                "total_invocations": e.total_invocations,
                "storage_bytes": e.storage_bytes,
                "buffer_bytes": e.buffer_bytes,
            })).collect::<Vec<_>>(),
            "effective": {
                "max_compute_workgroup_size_x": self.effective.max_compute_workgroup_size_x,
                "max_compute_workgroup_size_y": self.effective.max_compute_workgroup_size_y,
                "max_compute_workgroup_size_z": self.effective.max_compute_workgroup_size_z,
                "max_compute_invocations_per_workgroup": self.effective.max_compute_invocations_per_workgroup,
                "max_compute_workgroups_per_dimension": self.effective.max_compute_workgroups_per_dimension,
                "max_storage_buffer_binding_size": self.effective.max_storage_buffer_binding_size,
                "max_buffer_size": self.effective.max_buffer_size,
                "max_storage_buffers_per_shader_stage": self.effective.max_storage_buffers_per_shader_stage,
                "max_bind_groups": self.effective.max_bind_groups,
                "max_bindings_per_bind_group": self.effective.max_bindings_per_bind_group,
            },
        })
    }
}

fn effective_limits(device: &wgpu::Device, adapter: &wgpu::Adapter) -> wgpu::Limits {
    let mut effective = device.limits();
    let adapter_limits = adapter.limits();
    effective.max_compute_workgroup_size_x = effective
        .max_compute_workgroup_size_x
        .min(adapter_limits.max_compute_workgroup_size_x);
    effective.max_compute_workgroup_size_y = effective
        .max_compute_workgroup_size_y
        .min(adapter_limits.max_compute_workgroup_size_y);
    effective.max_compute_workgroup_size_z = effective
        .max_compute_workgroup_size_z
        .min(adapter_limits.max_compute_workgroup_size_z);
    effective.max_compute_invocations_per_workgroup = effective
        .max_compute_invocations_per_workgroup
        .min(adapter_limits.max_compute_invocations_per_workgroup);
    effective.max_compute_workgroups_per_dimension = effective
        .max_compute_workgroups_per_dimension
        .min(adapter_limits.max_compute_workgroups_per_dimension);
    effective.max_storage_buffer_binding_size = effective
        .max_storage_buffer_binding_size
        .min(adapter_limits.max_storage_buffer_binding_size);
    effective.max_buffer_size = effective
        .max_buffer_size
        .min(adapter_limits.max_buffer_size);
    effective.max_storage_buffers_per_shader_stage = effective
        .max_storage_buffers_per_shader_stage
        .min(adapter_limits.max_storage_buffers_per_shader_stage);
    effective.max_bind_groups = effective
        .max_bind_groups
        .min(adapter_limits.max_bind_groups);
    effective.max_bindings_per_bind_group = effective
        .max_bindings_per_bind_group
        .min(adapter_limits.max_bindings_per_bind_group);
    effective
}

fn logical_bytes_of(
    resources: &BTreeMap<String, ResourceV3>,
    width: u64,
    height: u64,
) -> Option<u64> {
    let mut total: u64 = 0;
    for resource in resources.values() {
        let cost = match resource {
            ResourceV3::Texture { format, .. } => width
                .checked_mul(height)?
                .checked_mul(format.bytes_per_texel())?,
            ResourceV3::Buffer { size_bytes, .. } => *size_bytes,
            ResourceV3::Sampler { .. } => 0,
        };
        total = total.checked_add(cost)?;
    }
    Some(total)
}

fn storage_staging_bytes_of(resources: &BTreeMap<String, ResourceV3>) -> Option<u64> {
    let mut total: u64 = 0;
    for resource in resources.values() {
        if let ResourceV3::Buffer { size_bytes, .. } = resource {
            total = total.checked_add(*size_bytes)?;
        }
    }
    Some(total)
}

pub fn transient_texture_bytes_of_resources(
    resources: &BTreeMap<String, ResourceV3>,
    width: u32,
    height: u32,
) -> Option<u64> {
    let mut total: u64 = 0;
    for resource in resources.values() {
        if let ResourceV3::Texture { format, .. } = resource {
            total = total.checked_add(
                u64::from(width)
                    .checked_mul(u64::from(height))?
                    .checked_mul(format.bytes_per_texel())?,
            )?;
        }
    }
    Some(total)
}

pub fn transient_texture_bytes(pack: &ValidatedPackV4, width: u32, height: u32) -> Option<u64> {
    transient_texture_bytes_of_resources(&pack.graph.resources, width, height)
}

fn estimate_peak_bytes_of(
    resources: &BTreeMap<String, ResourceV3>,
    width: u32,
    height: u32,
) -> Option<u64> {
    let logical = logical_bytes_of(resources, u64::from(width), u64::from(height))?;
    let padded = padded_final_staging_bytes(width, height)?;
    let staging = storage_staging_bytes_of(resources)?;
    logical.checked_add(padded)?.checked_add(staging)
}

fn estimate_resize_transition_peak_of(
    resources: &BTreeMap<String, ResourceV3>,
    old_size: [u32; 2],
    new_size: [u32; 2],
) -> Option<u64> {
    let old_textures = transient_texture_bytes_of_resources(resources, old_size[0], old_size[1])?;
    let new_peak = estimate_peak_bytes_of(resources, new_size[0], new_size[1])?;
    old_textures.checked_add(new_peak)
}

pub fn estimate_resize_transition_peak_bytes(
    pack: &ValidatedPackV4,
    old_size: [u32; 2],
    new_size: [u32; 2],
) -> Option<u64> {
    let base = estimate_resize_transition_peak_of(&pack.graph.resources, old_size, new_size)?
        .checked_add(if pack.params.is_some() { 1024 } else { 0 })?
        .checked_add(pack.shadow.map_or(0, |config| config.bytes()))?
        .checked_add(if pack.needs_entity_inputs() {
            crate::entities_v4::RESERVATION
        } else {
            0
        })?
        .checked_add(match &pack.history {
            Some(h) => crate::history_v4::reservation(h, old_size)?
                .checked_add(crate::history_v4::reservation(h, new_size)?)?,
            None => 0,
        })?;
    let base = base.checked_add(if pack.needs_light_inputs() {
        crate::frame_v4::light_reservation(old_size)?
            .checked_add(crate::frame_v4::light_reservation(new_size)?)?
    } else {
        0
    })?;
    if pack.needs_frame_inputs() {
        base.checked_add(crate::frame_v4::reservation(old_size)?)?
            .checked_add(crate::frame_v4::reservation(new_size)?)
    } else {
        Some(base)
    }
}

pub fn check_device_limits(
    pack: &ValidatedPackV4,
    effective: &wgpu::Limits,
) -> Result<V4DeviceLimitsReport> {
    check_device_limits_inner(
        &pack.profile,
        &pack.passes,
        &pack.programs,
        &pack.graph.resources,
        effective,
    )
}

fn check_device_limits_inner(
    profile: &str,
    passes: &[crate::graph_v4::BoundPassV4],
    programs: &BTreeMap<String, crate::graph_v4::ProgramV4>,
    resources: &BTreeMap<String, ResourceV3>,
    effective: &wgpu::Limits,
) -> Result<V4DeviceLimitsReport> {
    ensure(
        1 <= effective.max_bind_groups,
        "v4 device limit: v4 slice needs 1 bind group \
         but effective max_bind_groups is 0",
    )?;
    ensure(
        2 <= effective.max_bindings_per_bind_group,
        "v4 device limit: v4 post layout needs 2 bindings per group \
         but effective max_bindings_per_bind_group is less than 2",
    )?;
    let needs_compute = passes.iter().any(|p| p.stage == StageV3::Compute);
    let has_params = programs.values().any(|p| {
        matches!(
            p,
            crate::graph_v4::ProgramV4::Post {
                has_params: true,
                ..
            } | crate::graph_v4::ProgramV4::Lighting {
                has_params: true,
                ..
            }
        )
    });
    let has_shadow = programs
        .values()
        .any(|p| matches!(p, crate::graph_v4::ProgramV4::Post { shadow: true, .. }));
    let has_history = programs
        .values()
        .any(|p| matches!(p, crate::graph_v4::ProgramV4::Post { history: true, .. }));
    let has_light = programs.values().any(|p| {
        matches!(
            p,
            crate::graph_v4::ProgramV4::Post { light: true, .. }
                | crate::graph_v4::ProgramV4::Lighting { .. }
        )
    });
    if programs
        .values()
        .any(|p| matches!(p, crate::graph_v4::ProgramV4::Post { entities: true, .. }))
    {
        crate::entities_v4::check_device_with_light(effective, has_light)?;
    }
    if has_light {
        ensure(
            effective.max_bindings_per_bind_group >= 13,
            "v4 light binding12 exceeds bind-group limit",
        )?;
    }
    if has_history {
        ensure(
            effective.max_bindings_per_bind_group >= 12,
            "v4 temporal binding11 exceeds bind-group limit",
        )?;
    }
    if has_params {
        ensure(
            effective.max_uniform_buffer_binding_size >= 1024
                && effective.max_uniform_buffers_per_shader_stage >= 1
                && effective.max_bindings_per_bind_group >= 3,
            "v4 settings exceed uniform limits",
        )?;
    }
    if programs.values().any(|p| {
        matches!(
            p,
            crate::graph_v4::ProgramV4::Post {
                frame_inputs: true,
                ..
            } | crate::graph_v4::ProgramV4::Lighting { .. }
        )
    }) {
        ensure(
            effective.max_bindings_per_bind_group
                >= 7 + u32::from(has_params)
                    + if has_shadow { 2 } else { 0 }
                    + if has_history { 2 } else { 0 }
                    + u32::from(has_light)
                && effective.max_sampled_textures_per_shader_stage
                    >= 4 + u32::from(has_shadow) + u32::from(has_history) + u32::from(has_light)
                && effective.max_uniform_buffers_per_shader_stage
                    >= 2 + u32::from(has_params) + u32::from(has_shadow) + u32::from(has_history)
                && effective.max_uniform_buffer_binding_size >= if has_history { 192 } else { 176 },
            "v4 frame inputs exceed texture/uniform/bind-group limits",
        )?;
    }
    if needs_compute {
        ensure(
            1 <= effective.max_storage_buffers_per_shader_stage,
            "v4 device limit: v4 compute layout needs 1 storage buffer per stage \
             but effective max_storage_buffers_per_shader_stage is 0",
        )?;
    };
    let _ = profile;
    let mut entries = Vec::new();
    for pass in passes {
        if pass.stage != StageV3::Compute {
            continue;
        }
        let program = programs
            .get(&pass.program)
            .ok_or_else(|| format!("v4 prepares unknown program '{}'", pass.program))?;
        let (storage_bytes, workgroup_size) = match program {
            crate::graph_v4::ProgramV4::Compute {
                storage_bytes,
                workgroup_size,
                ..
            } => (*storage_bytes, *workgroup_size),
            crate::graph_v4::ProgramV4::Post { .. }
            | crate::graph_v4::ProgramV4::Lighting { .. } => {
                return Err("v4 program/pass stage mismatch".into());
            }
        };
        let dispatch = pass
            .dispatch
            .ok_or_else(|| format!("v4 compute pass '{}' has no dispatch", pass.id))?;
        let [wx, wy, wz] = workgroup_size;
        if wx > effective.max_compute_workgroup_size_x {
            return Err(format!(
                "v4 device limit: pass '{}' program '{}' workgroup_size x={wx} \
                 exceeds effective max_compute_workgroup_size_x={}",
                pass.id, pass.program, effective.max_compute_workgroup_size_x,
            ));
        }
        if wy > effective.max_compute_workgroup_size_y {
            return Err(format!(
                "v4 device limit: pass '{}' program '{}' workgroup_size y={wy} \
                 exceeds effective max_compute_workgroup_size_y={}",
                pass.id, pass.program, effective.max_compute_workgroup_size_y,
            ));
        }
        if wz > effective.max_compute_workgroup_size_z {
            return Err(format!(
                "v4 device limit: pass '{}' program '{}' workgroup_size z={wz} \
                 exceeds effective max_compute_workgroup_size_z={}",
                pass.id, pass.program, effective.max_compute_workgroup_size_z,
            ));
        }
        let invocations = u64::from(wx)
            .checked_mul(u64::from(wy))
            .and_then(|n| n.checked_mul(u64::from(wz)))
            .ok_or_else(|| {
                format!(
                    "v4 device-limit arithmetic overflow: pass '{}' program '{}' \
                     workgroup_size=[{wx}, {wy}, {wz}] product overflows u64",
                    pass.id, pass.program,
                )
            })?;
        if invocations > u64::from(effective.max_compute_invocations_per_workgroup) {
            return Err(format!(
                "v4 device limit: pass '{}' program '{}' invocations_per_workgroup={invocations} \
                 (workgroup_size=[{wx}, {wy}, {wz}]) exceeds effective \
                 max_compute_invocations_per_workgroup={}",
                pass.id, pass.program, effective.max_compute_invocations_per_workgroup,
            ));
        }
        for (axis, n) in ["x", "y", "z"].into_iter().zip(dispatch) {
            if n > effective.max_compute_workgroups_per_dimension {
                return Err(format!(
                    "v4 device limit: pass '{}' program '{}' dispatch_{axis}={n} \
                     (dispatch=[{}, {}, {}]) exceeds effective \
                     max_compute_workgroups_per_dimension={}",
                    pass.id,
                    pass.program,
                    dispatch[0],
                    dispatch[1],
                    dispatch[2],
                    effective.max_compute_workgroups_per_dimension,
                ));
            }
        }
        let dispatch_product = u64::from(dispatch[0])
            .checked_mul(u64::from(dispatch[1]))
            .and_then(|n| n.checked_mul(u64::from(dispatch[2])))
            .ok_or_else(|| {
                format!(
                    "v4 device-limit arithmetic overflow: pass '{}' program '{}' \
                     dispatch=[{}, {}, {}] product overflows u64",
                    pass.id, pass.program, dispatch[0], dispatch[1], dispatch[2],
                )
            })?;
        let total = invocations.checked_mul(dispatch_product).ok_or_else(|| {
            format!(
                "v4 device-limit arithmetic overflow: pass '{}' program '{}' \
                 total_invocations={invocations}*{dispatch_product} overflows u64",
                pass.id, pass.program,
            )
        })?;
        let buffer_bytes = match resources.get(&pass.output) {
            Some(ResourceV3::Buffer { size_bytes, .. }) => *size_bytes,
            _ => {
                return Err(format!(
                    "v4 pass '{}' writes no storage buffer resource '{}'",
                    pass.id, pass.output,
                ));
            }
        };
        if storage_bytes > u64::from(effective.max_storage_buffer_binding_size) {
            return Err(format!(
                "v4 device limit: pass '{}' program '{}' resource '{}' \
                 storage_bytes={storage_bytes} exceeds effective \
                 max_storage_buffer_binding_size={}",
                pass.id, pass.program, pass.output, effective.max_storage_buffer_binding_size,
            ));
        }
        if buffer_bytes > u64::from(effective.max_storage_buffer_binding_size) {
            return Err(format!(
                "v4 device limit: pass '{}' resource '{}' buffer_bytes={buffer_bytes} \
                 exceeds effective max_storage_buffer_binding_size={}",
                pass.id, pass.output, effective.max_storage_buffer_binding_size,
            ));
        }
        if storage_bytes > effective.max_buffer_size {
            return Err(format!(
                "v4 device limit: pass '{}' program '{}' resource '{}' \
                 storage_bytes={storage_bytes} exceeds effective max_buffer_size={}",
                pass.id, pass.program, pass.output, effective.max_buffer_size,
            ));
        }
        if buffer_bytes > effective.max_buffer_size {
            return Err(format!(
                "v4 device limit: pass '{}' resource '{}' buffer_bytes={buffer_bytes} \
                 exceeds effective max_buffer_size={}",
                pass.id, pass.output, effective.max_buffer_size,
            ));
        }
        entries.push(V4ComputeLimitEntry {
            pass: pass.id.clone(),
            program: pass.program.clone(),
            resource: pass.output.clone(),
            workgroup_size,
            invocations_per_workgroup: invocations,
            dispatch,
            total_invocations: total,
            storage_bytes,
            buffer_bytes,
        });
    }
    Ok(V4DeviceLimitsReport {
        effective: effective.clone(),
        entries,
    })
}

impl RuntimeV4 {
    pub fn observe_history_context(&mut self, world: Option<u64>, camera_cut: u64) {
        if self.lighting.is_some() {
            self.lighting_phase = LightingPhase::Idle;
            if let Some(frame) = &mut self.frame {
                frame.ready = false;
            }
        }
        if let Some(history) = &mut self.history {
            history.observe(world, camera_cut);
        }
    }
    pub fn commit_submitted_frame(&mut self) {
        if let Some(h) = &mut self.history {
            h.commit();
        }
    }
    pub fn abort_frame(&mut self) {
        self.lighting_phase = LightingPhase::Idle;
        if self.lighting.is_some()
            && let Some(frame) = &mut self.frame
        {
            frame.ready = false;
        }
        if let Some(h) = &mut self.history {
            h.abort();
        }
    }
    pub fn reset_history(&mut self) {
        if let Some(h) = &mut self.history {
            h.reset();
        }
    }
    pub fn shadow_config(&self) -> Option<crate::shadow_v4::ShadowConfig> {
        self.shadow_config
    }

    pub fn encode_terrain_shadow(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: &crate::shadow_v4::ShadowFrame,
        atlas: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        draws: &[crate::shadow_v4::TerrainShadowDraw<'_>],
    ) -> Result<()> {
        self.shadow
            .as_mut()
            .ok_or("pack has no terrain shadow")?
            .encode(device, queue, encoder, frame, atlas, sampler, draws)
    }

    pub fn shadow_texture(&self) -> Option<&wgpu::Texture> {
        self.shadow.as_ref().map(|shadow| &shadow.texture)
    }
    pub fn entity_shadow_target(&self) -> Option<&wgpu::TextureView> {
        self.shadow
            .as_ref()
            .filter(|s| self.entities && s.ready)
            .map(|s| &s.view)
    }
    pub fn needs_entity_inputs(&self) -> bool {
        self.entities
    }
    pub fn needs_frame_inputs(&self) -> bool {
        self.frame.is_some()
    }
    pub fn needs_light_inputs(&self) -> bool {
        self.light
    }
    pub fn needs_lighting_stage(&self) -> bool {
        self.lighting.is_some()
    }

    pub fn capture_world_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        depth: &wgpu::TextureView,
        camera: &crate::model::FrameData,
        environment: &[u8; 80],
    ) -> Result<()> {
        ensure(
            self.lighting.is_none() || self.lighting_phase == LightingPhase::Idle,
            "terrain lighting frame already captured; abort before starting another frame",
        )?;
        self.frame
            .as_mut()
            .ok_or("v4 pack does not use frame inputs")?
            .capture(device, queue, encoder, depth, camera, environment)?;
        if let Some(history) = &mut self.history {
            history.begin(queue, encoder, camera)?;
        }
        if self.lighting.is_some() {
            self.lighting_phase = LightingPhase::Captured;
        }
        Ok(())
    }
    pub async fn prepare(
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        pack: &ValidatedPackV4,
        size: [u32; 2],
        scene_format: wgpu::TextureFormat,
        budget: u64,
    ) -> Result<Self> {
        use wgpu::{TextureFormat as F, TextureUsages as U};
        if matches!(scene_format, F::Rgba8UnormSrgb | F::Bgra8UnormSrgb) {
            return Err("v4 slice rejects sRGB scenes (no sRGB codec in this slice)".into());
        }
        ensure(
            matches!(scene_format, F::Rgba8Unorm | F::Bgra8Unorm),
            "v4 slice executes Rgba8Unorm/Bgra8Unorm scenes only (no sRGB codec in this slice)",
        )?;
        ensure(size.iter().all(|&n| n > 0), "v4 viewport must be nonzero")?;
        let limit = device
            .limits()
            .max_texture_dimension_2d
            .min(adapter.limits().max_texture_dimension_2d);
        ensure(
            size.iter().all(|&n| n <= limit),
            "v4 viewport exceeds effective GPU limits",
        )?;
        ensure(
            pack.min_texture_dimension <= limit,
            "v4 pack minimum GPU dimension",
        )?;
        let support = |format, usage, filter: bool| {
            let features = adapter.get_texture_format_features(format);
            features.allowed_usages.contains(usage)
                && (!filter
                    || features
                        .flags
                        .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE))
        };
        ensure(
            support(
                F::Rgba8Unorm,
                U::RENDER_ATTACHMENT | U::TEXTURE_BINDING | U::COPY_SRC,
                true,
            ),
            "v4 RGBA8 sample/filter/render unavailable",
        )?;
        ensure(
            support(scene_format, U::TEXTURE_BINDING, true),
            "v4 scene sample/filter unavailable for the admitted scene format",
        )?;
        if pack.lighting.is_some() {
            ensure(
                support(scene_format, U::RENDER_ATTACHMENT, false),
                "terrain lighting requires a renderable host scene format",
            )?;
        }
        for resource in pack.graph.resources.values() {
            if let ResourceV3::Texture { format, .. } = resource {
                ensure(
                    support(
                        gpu_format(*format),
                        U::RENDER_ATTACHMENT | U::TEXTURE_BINDING | U::COPY_SRC,
                        true,
                    ),
                    "v4 transient texture format unsupported",
                )?;
            }
        }
        if let Some(h) = &pack.history {
            ensure(
                support(
                    gpu_format(h.format),
                    U::TEXTURE_BINDING | U::COPY_DST | U::RENDER_ATTACHMENT,
                    true,
                ),
                "history format unsupported",
            )?;
        }
        if pack.needs_frame_inputs() {
            ensure(
                support(
                    F::Rgba16Float,
                    U::RENDER_ATTACHMENT | U::TEXTURE_BINDING,
                    true,
                ) && support(
                    F::Depth32Float,
                    U::RENDER_ATTACHMENT | U::TEXTURE_BINDING,
                    false,
                ),
                "v4 frame geometry/depth formats unsupported",
            )?;
        }
        if let Some(shadow) = pack.shadow {
            ensure(
                shadow.resolution <= limit,
                "shadow resolution exceeds effective texture dimension",
            )?;
            ensure(
                support(
                    F::Depth32Float,
                    U::RENDER_ATTACHMENT | U::TEXTURE_BINDING | U::COPY_SRC,
                    false,
                ),
                "shadow depth format unsupported",
            )?;
        }
        if pack.needs_light_inputs() {
            ensure(
                support(
                    F::Rg16Float,
                    U::RENDER_ATTACHMENT | U::TEXTURE_BINDING,
                    true,
                ),
                "v4 block/sky RG16Float sample/filter/render unavailable",
            )?;
        }
        if pack.needs_compute() && adapter.get_info().backend == wgpu::Backend::Gl {
            return Err(
                "v4 compute needs a supporting device; WebGL2 fallback admits post-only v4 packs"
                    .into(),
            );
        }
        let device_limits = check_device_limits(pack, &effective_limits(device, adapter))?;
        let logical = pack
            .logical_bytes(u64::from(size[0]), u64::from(size[1]))
            .ok_or("v4 budget arithmetic overflow")?;
        let peak =
            estimate_peak_bytes(pack, size[0], size[1]).ok_or("v4 budget arithmetic overflow")?;
        ensure(peak <= budget, "v4 pack exceeds budget")?;

        device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let outcome = Self::build(
            device,
            pack,
            size,
            logical,
            peak,
            budget,
            scene_format,
            device_limits,
        );
        let validation = device.pop_error_scope().await;
        let oom = device.pop_error_scope().await;
        if let Some(error) = validation.or(oom) {
            return Err(format!("v4 GPU preparation: {error}"));
        }
        outcome
    }

    fn build(
        device: &wgpu::Device,
        pack: &ValidatedPackV4,
        size: [u32; 2],
        logical: u64,
        peak: u64,
        budget: u64,
        scene_format: wgpu::TextureFormat,
        device_limits: V4DeviceLimitsReport,
    ) -> Result<Self> {
        let extent = wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        };
        let mut textures: BTreeMap<String, Target> = BTreeMap::new();
        let mut buffers: BTreeMap<String, wgpu::Buffer> = BTreeMap::new();
        let mut buffer_sizes: BTreeMap<String, u64> = BTreeMap::new();
        for (name, resource) in &pack.graph.resources {
            match resource {
                ResourceV3::Texture { format, .. } => {
                    let texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some(name),
                        size: extent,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: gpu_format(*format),
                        usage: wgpu::TextureUsages::TEXTURE_BINDING
                            | wgpu::TextureUsages::RENDER_ATTACHMENT
                            | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    });
                    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                    textures.insert(name.clone(), Target { texture, view });
                }
                ResourceV3::Buffer { size_bytes, .. } => {
                    buffers.insert(
                        name.clone(),
                        device.create_buffer(&wgpu::BufferDescriptor {
                            label: Some(name),
                            size: *size_bytes,
                            usage: wgpu::BufferUsages::STORAGE
                                | wgpu::BufferUsages::COPY_SRC
                                | wgpu::BufferUsages::COPY_DST,
                            mapped_at_creation: false,
                        }),
                    );
                    buffer_sizes.insert(name.clone(), *size_bytes);
                }
                ResourceV3::Sampler { .. } => {}
            }
        }
        let samplers = pack
            .graph
            .resources
            .iter()
            .filter_map(|(name, resource)| match resource {
                ResourceV3::Sampler { .. } => Some((
                    name.clone(),
                    device.create_sampler(&wgpu::SamplerDescriptor {
                        label: Some(name),
                        mag_filter: wgpu::FilterMode::Linear,
                        min_filter: wgpu::FilterMode::Linear,
                        mipmap_filter: wgpu::FilterMode::Nearest,
                        address_mode_u: wgpu::AddressMode::ClampToEdge,
                        address_mode_v: wgpu::AddressMode::ClampToEdge,
                        address_mode_w: wgpu::AddressMode::ClampToEdge,
                        ..Default::default()
                    }),
                )),
                _ => None,
            })
            .collect();

        let fragment = wgpu::ShaderStages::FRAGMENT;
        let mut post_entries = vec![
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: fragment,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: fragment,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ];
        if pack.params.is_some() {
            post_entries.push(crate::frame_v4::params_entry());
        }
        let post_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("v4_post"),
            entries: &post_entries,
        });
        let params = pack.params.as_ref().map(|bytes| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("v4 pack settings"),
                size: 1024,
                usage: wgpu::BufferUsages::UNIFORM,
                mapped_at_creation: true,
            });
            buffer
                .slice(..)
                .get_mapped_range_mut()
                .copy_from_slice(bytes);
            buffer.unmap();
            buffer
        });
        let compute_layout = if pack.needs_compute() {
            Some(
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("v4_compute"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                }),
            )
        } else {
            None
        };
        let frame = pack.needs_frame_inputs().then(|| {
            crate::frame_v4::FrameGpu::new(
                device,
                size,
                params.is_some(),
                pack.shadow.is_some(),
                pack.history.is_some(),
                pack.needs_light_inputs(),
            )
        });
        if pack.needs_light_inputs() {
            crate::light_v4::probe(device, pack.needs_entity_inputs())?;
        }
        let history = pack
            .history
            .clone()
            .map(|config| crate::history_v4::HistoryGpu::new(device, config, size));
        let shadow = pack
            .shadow
            .map(|config| crate::shadow_v4::ShadowGpu::new(device, config));
        let post_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("v4_post_pl"),
            bind_group_layouts: &[&post_layout],
            push_constant_ranges: &[],
        });
        let compute_pipeline_layout = compute_layout.as_ref().map(|layout| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("v4_compute_pl"),
                bind_group_layouts: &[layout],
                push_constant_ranges: &[],
            })
        });
        let frame_pipeline_layout = frame.as_ref().map(|frame| {
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("v4 world frame pipeline"),
                bind_group_layouts: &[&frame.layout],
                push_constant_ranges: &[],
            })
        });

        let mut post_pipes: BTreeMap<String, wgpu::RenderPipeline> = BTreeMap::new();
        let mut compute_pipes: BTreeMap<String, wgpu::ComputePipeline> = BTreeMap::new();
        for pass in &pack.passes {
            let program = pack
                .programs
                .get(&pass.program)
                .ok_or_else(|| format!("v4 prepares unknown program '{}'", pass.program))?;
            match (pass.stage, program) {
                (
                    StageV3::Post,
                    crate::graph_v4::ProgramV4::Post {
                        source,
                        vertex_entry,
                        fragment_entry,
                        frame_inputs,
                        ..
                    },
                ) => {
                    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(&pass.program),
                        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
                    });
                    let targets = [Some(wgpu::ColorTargetState {
                        format: match pack.graph.resources.get(&pass.output) {
                            Some(ResourceV3::Texture { format, .. }) => gpu_format(*format),
                            _ => return Err("v4 post target format missing".into()),
                        },
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })];
                    post_pipes.insert(
                        pass.id.clone(),
                        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                            label: Some(&pass.program),
                            layout: Some(if *frame_inputs {
                                frame_pipeline_layout
                                    .as_ref()
                                    .ok_or("v4 frame pipeline missing")?
                            } else {
                                &post_pipeline_layout
                            }),
                            vertex: wgpu::VertexState {
                                module: &module,
                                entry_point: vertex_entry,
                                buffers: &[],
                            },
                            fragment: Some(wgpu::FragmentState {
                                module: &module,
                                entry_point: fragment_entry,
                                targets: &targets,
                            }),
                            primitive: wgpu::PrimitiveState {
                                topology: wgpu::PrimitiveTopology::TriangleList,
                                strip_index_format: None,
                                front_face: wgpu::FrontFace::Ccw,
                                cull_mode: None,
                                polygon_mode: wgpu::PolygonMode::Fill,
                                unclipped_depth: false,
                                conservative: false,
                            },
                            depth_stencil: None,
                            multisample: wgpu::MultisampleState {
                                count: 1,
                                mask: !0,
                                alpha_to_coverage_enabled: false,
                            },
                            multiview: None,
                        }),
                    );
                }
                (
                    StageV3::Lighting,
                    crate::graph_v4::ProgramV4::Lighting {
                        source,
                        vertex_entry,
                        fragment_entry,
                        ..
                    },
                ) => {
                    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(&pass.program),
                        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
                    });
                    let format = match pack.graph.resources.get(&pass.output) {
                        Some(ResourceV3::Texture { format, .. }) => gpu_format(*format),
                        _ => return Err("v4 lighting target format missing".into()),
                    };
                    post_pipes.insert(
                        pass.id.clone(),
                        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                            label: Some(&pass.program),
                            layout: Some(
                                frame_pipeline_layout
                                    .as_ref()
                                    .ok_or("v4 lighting frame pipeline missing")?,
                            ),
                            vertex: wgpu::VertexState {
                                module: &module,
                                entry_point: vertex_entry,
                                buffers: &[],
                            },
                            fragment: Some(wgpu::FragmentState {
                                module: &module,
                                entry_point: fragment_entry,
                                targets: &[Some(wgpu::ColorTargetState {
                                    format,
                                    blend: None,
                                    write_mask: wgpu::ColorWrites::ALL,
                                })],
                            }),
                            primitive: wgpu::PrimitiveState::default(),
                            depth_stencil: None,
                            multisample: Default::default(),
                            multiview: None,
                        }),
                    );
                }
                (
                    StageV3::Compute,
                    crate::graph_v4::ProgramV4::Compute {
                        source,
                        compute_entry,
                        storage_bytes,
                        ..
                    },
                ) => {
                    if compute_pipes.contains_key(&pass.program) {
                        continue;
                    }
                    ensure(
                        buffer_sizes.get(&pass.output) == Some(storage_bytes),
                        "v4 compute buffer/program size mismatch",
                    )?;
                    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                        label: Some(&pass.program),
                        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
                    });
                    let pipeline_layout = compute_pipeline_layout.as_ref().ok_or_else(|| {
                        format!(
                            "v4 pass '{}': compute pipeline layout missing (post-only runtime)",
                            pass.id
                        )
                    })?;
                    compute_pipes.insert(
                        pass.program.clone(),
                        device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some(&pass.program),
                            layout: Some(pipeline_layout),
                            module: &module,
                            entry_point: compute_entry,
                        }),
                    );
                }
                _ => return Err("v4 program/pass stage mismatch".into()),
            }
        }

        Ok(Self {
            profile: pack.profile.clone(),
            min_texture_dimension: pack.min_texture_dimension,
            passes: pack.passes.clone(),
            programs: pack.programs.clone(),
            resources: pack.graph.resources.clone(),
            final_output: pack
                .graph
                .final_outputs
                .first()
                .cloned()
                .ok_or("v4 no final")?,
            textures,
            buffers,
            buffer_sizes,
            samplers,
            post_pipes,
            compute_pipes,
            post_layout,
            frame,
            params,
            shadow,
            shadow_config: pack.shadow,
            entities: pack.needs_entity_inputs(),
            light: pack.needs_light_inputs(),
            lighting: pack.lighting.as_ref().map(|config| {
                crate::lighting_v4::LightingGpu::new(device, config.output.clone(), scene_format)
            }),
            lighting_phase: LightingPhase::Idle,
            history,
            compute_layout,
            size,
            logical,
            bytes: peak,
            budget,
            frames_run: 0,
            generation: 0,
            scene_format,
            device_limits,
        })
    }

    fn input_view<'a>(
        &'a self,
        pass_id: &str,
        name: &str,
        scene: &'a wgpu::TextureView,
    ) -> Result<&'a wgpu::TextureView> {
        if name == "engine.scene_color" || name == "engine.world_color" {
            Ok(scene)
        } else {
            self.textures
                .get(name)
                .map(|target| &target.view)
                .ok_or_else(|| format!("v4 pass '{pass_id}': unknown texture '{name}'"))
        }
    }

    pub fn encode(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene: &wgpu::TextureView,
    ) -> Result<()> {
        self.encode_with_geometry(device, encoder, scene, None)
    }

    pub fn encode_with_geometry(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene: &wgpu::TextureView,
        geometry: Option<(&wgpu::TextureView, &wgpu::TextureView)>,
    ) -> Result<()> {
        self.encode_with_geometry_and_light(device, encoder, scene, geometry, None)
    }

    pub fn encode_with_geometry_and_light(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene: &wgpu::TextureView,
        geometry: Option<(&wgpu::TextureView, &wgpu::TextureView)>,
        light: Option<&wgpu::TextureView>,
    ) -> Result<()> {
        ensure(
            self.lighting.is_none(),
            "terrain lighting requires separate early and remaining encode phases",
        )?;
        self.encode_passes(device, encoder, scene, geometry, light, None)?;
        self.complete_frame(encoder)
    }

    pub fn encode_lighting(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene: &wgpu::TextureView,
        geometry: Option<(&wgpu::TextureView, &wgpu::TextureView)>,
        light: Option<&wgpu::TextureView>,
    ) -> Result<()> {
        ensure(
            self.lighting.is_some(),
            "pack has no authored terrain lighting stage",
        )?;
        ensure(
            self.lighting_phase == LightingPhase::Captured,
            "terrain lighting requires a fresh opaque-prefix capture and may encode once",
        )?;
        self.encode_passes(
            device,
            encoder,
            scene,
            geometry,
            light,
            Some(StageV3::Lighting),
        )?;
        let lighting = self
            .lighting
            .as_ref()
            .ok_or("terrain lighting host composition missing")?;
        let color = self
            .textures
            .get(&lighting.output)
            .ok_or("terrain lighting boundary output missing")?;
        lighting.compose(device, encoder, &color.view, scene);
        self.lighting_phase = LightingPhase::Composed;
        Ok(())
    }

    pub fn encode_remaining(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene: &wgpu::TextureView,
    ) -> Result<()> {
        ensure(
            self.lighting.is_some() && self.lighting_phase == LightingPhase::Composed,
            "remaining lighting-pack passes require this frame's composed terrain lighting",
        )?;
        self.encode_passes(device, encoder, scene, None, None, Some(StageV3::Post))?;
        self.complete_frame(encoder)
    }

    fn encode_passes(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        scene: &wgpu::TextureView,
        geometry: Option<(&wgpu::TextureView, &wgpu::TextureView)>,
        light: Option<&wgpu::TextureView>,
        phase: Option<StageV3>,
    ) -> Result<()> {
        let frame_phase = phase != Some(StageV3::Post);
        ensure(
            !frame_phase || !self.light || light.is_some(),
            "v4 opaque block/sky light was not captured for this frame",
        )?;
        if frame_phase && let Some(frame) = &self.frame {
            ensure(
                frame.ready && geometry.is_some(),
                "v4 frame inputs were not captured for this frame",
            )?;
        }
        if let Some(shadow) = &self.shadow {
            ensure(shadow.ready, "terrain shadow not captured for this frame")?;
        }
        for pass in &self.passes {
            if phase.is_some_and(|stage| pass.stage != stage) {
                continue;
            }
            match pass.stage {
                StageV3::Post | StageV3::Lighting => {
                    let input = pass
                        .input
                        .as_ref()
                        .ok_or_else(|| format!("v4 post pass '{}' has no input", pass.id))?;
                    let sampler_name = pass
                        .sampler
                        .as_ref()
                        .ok_or_else(|| format!("v4 post pass '{}' has no sampler", pass.id))?;
                    let input_view = self.input_view(&pass.id, input, scene)?;
                    let sampler = self.samplers.get(sampler_name).ok_or_else(|| {
                        format!("v4 pass '{}': unknown sampler '{sampler_name}'", pass.id)
                    })?;
                    let output = self.textures.get(&pass.output).ok_or_else(|| {
                        format!("v4 pass '{}': unknown texture '{}'", pass.id, pass.output)
                    })?;
                    let pipeline = self.post_pipes.get(&pass.id).ok_or_else(|| {
                        format!("v4 pass '{}': missing pipeline '{}'", pass.id, pass.program)
                    })?;
                    let mut entries = vec![
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(input_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(sampler),
                        },
                    ];
                    if let Some(params) = &self.params {
                        entries.push(wgpu::BindGroupEntry {
                            binding: 2,
                            resource: params.as_entire_binding(),
                        });
                    }
                    let wants_frame = matches!(
                        self.programs.get(&pass.program),
                        Some(crate::graph_v4::ProgramV4::Post {
                            frame_inputs: true,
                            ..
                        }) | Some(crate::graph_v4::ProgramV4::Lighting { .. })
                    );
                    let layout = if wants_frame {
                        let frame = self.frame.as_ref().ok_or("v4 frame buffers missing")?;
                        let (normal, albedo) = geometry.ok_or("v4 opaque geometry missing")?;
                        if self.light {
                            entries.push(wgpu::BindGroupEntry {
                                binding: 12,
                                resource: wgpu::BindingResource::TextureView(
                                    light.ok_or("v4 opaque block/sky light missing")?,
                                ),
                            });
                        }
                        entries.extend([
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::TextureView(&frame.depth),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: frame.camera.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 5,
                                resource: frame.environment.as_entire_binding(),
                            },
                            wgpu::BindGroupEntry {
                                binding: 6,
                                resource: wgpu::BindingResource::TextureView(normal),
                            },
                            wgpu::BindGroupEntry {
                                binding: 7,
                                resource: wgpu::BindingResource::TextureView(albedo),
                            },
                        ]);
                        if let Some(shadow) = &self.shadow {
                            entries.extend([
                                wgpu::BindGroupEntry {
                                    binding: 8,
                                    resource: wgpu::BindingResource::TextureView(&shadow.view),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 9,
                                    resource: shadow.uniform.as_entire_binding(),
                                },
                            ]);
                        }
                        if let Some(history) = &self.history {
                            entries.extend([
                                wgpu::BindGroupEntry {
                                    binding: 10,
                                    resource: wgpu::BindingResource::TextureView(history.view()),
                                },
                                wgpu::BindGroupEntry {
                                    binding: 11,
                                    resource: history.uniform.as_entire_binding(),
                                },
                            ]);
                        }
                        &frame.layout
                    } else {
                        &self.post_layout
                    };
                    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(pass.id.as_str()),
                        layout,
                        entries: &entries,
                    });
                    let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some(pass.id.as_str()),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &output.view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                    render.set_pipeline(pipeline);
                    render.set_bind_group(0, &group, &[]);
                    render.draw(0..3, 0..1);
                }
                StageV3::Compute => {
                    let buffer = self.buffers.get(&pass.output).ok_or_else(|| {
                        format!("v4 pass '{}': unknown buffer '{}'", pass.id, pass.output)
                    })?;
                    let pipeline = self.compute_pipes.get(&pass.program).ok_or_else(|| {
                        format!("v4 pass '{}': missing pipeline '{}'", pass.id, pass.program)
                    })?;
                    let layout = self.compute_layout.as_ref().ok_or_else(|| {
                        format!(
                            "v4 pass '{}': compute layout missing (post-only runtime)",
                            pass.id
                        )
                    })?;
                    let size = self
                        .buffer_sizes
                        .get(&pass.output)
                        .copied()
                        .ok_or_else(|| {
                            format!("v4 pass '{}': unknown buffer '{}'", pass.id, pass.output)
                        })?;
                    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some(pass.id.as_str()),
                        layout,
                        entries: &[wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                buffer,
                                offset: 0,
                                size: nonzero(size),
                            }),
                        }],
                    });
                    let dispatch = pass
                        .dispatch
                        .ok_or_else(|| format!("v4 compute pass '{}' has no dispatch", pass.id))?;
                    let mut compute = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some(pass.id.as_str()),
                        timestamp_writes: None,
                    });
                    compute.set_pipeline(pipeline);
                    compute.set_bind_group(0, &group, &[]);
                    compute.dispatch_workgroups(dispatch[0], dispatch[1], dispatch[2]);
                }
                StageV3::Geometry | StageV3::Shadow => {
                    return Err(format!(
                        "v4 pass '{}': stage '{}' never prepares (no GPU hook)",
                        pass.id,
                        pass.stage.as_str()
                    ));
                }
            }
        }
        Ok(())
    }

    fn complete_frame(&mut self, encoder: &mut wgpu::CommandEncoder) -> Result<()> {
        if let Some(history) = &mut self.history {
            let source = self
                .textures
                .get(&history.config.source)
                .ok_or("history source missing")?;
            history.copy_current(encoder, &source.texture)?;
        }
        if let Some(frame) = &mut self.frame {
            frame.ready = false;
        }
        if let Some(shadow) = &mut self.shadow {
            shadow.ready = false;
        }
        self.frames_run += 1;
        self.lighting_phase = LightingPhase::Idle;
        Ok(())
    }

    pub fn final_view(&self) -> Result<&wgpu::TextureView> {
        self.textures
            .get(&self.final_output)
            .map(|target| &target.view)
            .ok_or_else(|| {
                format!(
                    "v4 pack '{}': unknown final texture '{}'",
                    self.profile, self.final_output
                )
            })
    }

    pub fn final_texture(&self) -> Result<&wgpu::Texture> {
        self.textures
            .get(&self.final_output)
            .map(|target| &target.texture)
            .ok_or_else(|| format!("v4 missing final texture '{}'", self.final_output))
    }

    pub fn dimensions(&self) -> [u32; 2] {
        self.size
    }

    pub fn frames_run(&self) -> u64 {
        self.frames_run
    }

    pub async fn resize(
        &mut self,
        device: &wgpu::Device,
        adapter: &wgpu::Adapter,
        new_size: [u32; 2],
        new_budget: u64,
    ) -> Result<()> {
        use wgpu::{TextureFormat as F, TextureUsages as U};
        let profile = self.profile.clone();
        if !new_size.iter().all(|&n| n > 0) {
            return Err(format!(
                "v4 resize pack '{profile}': viewport must be nonzero"
            ));
        }
        let limit = device
            .limits()
            .max_texture_dimension_2d
            .min(adapter.limits().max_texture_dimension_2d);
        if !new_size.iter().all(|&n| n <= limit) {
            return Err(format!(
                "v4 resize pack '{profile}': viewport [{} x {}] exceeds effective GPU limit {limit}",
                new_size[0], new_size[1],
            ));
        }
        if !(self.min_texture_dimension <= limit) {
            return Err(format!(
                "v4 resize pack '{profile}': v4 pack minimum GPU dimension"
            ));
        }
        let support = |format, usage, filter: bool| {
            let features = adapter.get_texture_format_features(format);
            features.allowed_usages.contains(usage)
                && (!filter
                    || features
                        .flags
                        .contains(wgpu::TextureFormatFeatureFlags::FILTERABLE))
        };
        if !support(
            F::Rgba8Unorm,
            U::RENDER_ATTACHMENT | U::TEXTURE_BINDING | U::COPY_SRC,
            true,
        ) {
            return Err(format!(
                "v4 resize pack '{profile}': v4 RGBA8 sample/filter/render unavailable"
            ));
        }
        if !support(self.scene_format, U::TEXTURE_BINDING, true) {
            return Err(format!(
                "v4 resize pack '{profile}': v4 scene sample/filter unavailable \
                 for the admitted scene format"
            ));
        }
        if self.lighting.is_some() && !support(self.scene_format, U::RENDER_ATTACHMENT, false) {
            return Err("v4 lighting resize requires a renderable host scene format".into());
        }
        for resource in self.resources.values() {
            if let ResourceV3::Texture { format, .. } = resource {
                ensure(
                    support(
                        gpu_format(*format),
                        U::RENDER_ATTACHMENT | U::TEXTURE_BINDING | U::COPY_SRC,
                        true,
                    ),
                    "v4 resize transient texture format unsupported",
                )?;
            }
        }
        if self.frame.is_some() {
            ensure(
                support(
                    F::Rgba16Float,
                    U::RENDER_ATTACHMENT | U::TEXTURE_BINDING,
                    true,
                ) && support(
                    F::Depth32Float,
                    U::RENDER_ATTACHMENT | U::TEXTURE_BINDING,
                    false,
                ),
                "v4 resize frame geometry/depth formats unsupported",
            )?;
        }
        if self.light {
            ensure(
                support(
                    F::Rg16Float,
                    U::RENDER_ATTACHMENT | U::TEXTURE_BINDING,
                    true,
                ),
                "v4 resize block/sky RG16Float sample/filter/render unavailable",
            )?;
        }
        if let Some(h) = &self.history {
            ensure(
                support(
                    gpu_format(h.config.format),
                    U::TEXTURE_BINDING | U::COPY_DST | U::RENDER_ATTACHMENT,
                    true,
                ),
                "history resize format unsupported",
            )?;
        }
        let needs_compute = self.passes.iter().any(|p| p.stage == StageV3::Compute);
        if needs_compute && adapter.get_info().backend == wgpu::Backend::Gl {
            return Err(format!(
                "v4 resize pack '{profile}': v4 compute needs a supporting device; \
                 WebGL2 fallback admits post-only v4 packs"
            ));
        }
        let device_limits = check_device_limits_inner(
            &profile,
            &self.passes,
            &self.programs,
            &self.resources,
            &effective_limits(device, adapter),
        )
        .map_err(|error| format!("v4 resize pack '{profile}': {error}"))?;
        let logical = logical_bytes_of(
            &self.resources,
            u64::from(new_size[0]),
            u64::from(new_size[1]),
        )
        .ok_or_else(|| format!("v4 resize pack '{profile}': v4 budget arithmetic overflow"))?;
        let extra = if self.frame.is_some() {
            crate::frame_v4::reservation(new_size).ok_or("v4 frame budget overflow")?
        } else {
            0
        } + if self.params.is_some() { 1024 } else { 0 }
            + if self.light {
                crate::frame_v4::light_reservation(new_size).ok_or("v4 light budget overflow")?
            } else {
                0
            }
            + self.shadow_config.map_or(0, |config| config.bytes())
            + if self.entities {
                crate::entities_v4::RESERVATION
            } else {
                0
            }
            + match &self.history {
                Some(h) => crate::history_v4::reservation(&h.config, new_size)
                    .ok_or("history budget overflow")?,
                None => 0,
            };
        let old_extra = if self.frame.is_some() {
            crate::frame_v4::reservation(self.size).ok_or("v4 frame budget overflow")?
        } else {
            0
        } + match &self.history {
            Some(h) => crate::history_v4::reservation(&h.config, self.size)
                .ok_or("history budget overflow")?,
            None => 0,
        } + if self.light {
            crate::frame_v4::light_reservation(self.size).ok_or("v4 light budget overflow")?
        } else {
            0
        };
        let peak = estimate_peak_bytes_of(&self.resources, new_size[0], new_size[1])
            .and_then(|n| n.checked_add(extra))
            .ok_or_else(|| format!("v4 resize pack '{profile}': v4 budget arithmetic overflow"))?;
        let transition = estimate_resize_transition_peak_of(&self.resources, self.size, new_size)
            .and_then(|n| n.checked_add(extra)?.checked_add(old_extra))
            .ok_or_else(|| format!("v4 resize pack '{profile}': v4 budget arithmetic overflow"))?;
        if !(transition <= new_budget) {
            return Err(format!(
                "v4 resize pack '{profile}': v4 pack exceeds transition budget \
                 (transition peak {transition} > budget {new_budget}; \
                 new steady-state peak {peak})"
            ));
        }
        let extent = wgpu::Extent3d {
            width: new_size[0],
            height: new_size[1],
            depth_or_array_layers: 1,
        };
        device.push_error_scope(wgpu::ErrorFilter::OutOfMemory);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let mut replacement: BTreeMap<String, Target> = BTreeMap::new();
        for (name, resource) in &self.resources {
            if let ResourceV3::Texture { format, .. } = resource {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(name),
                    size: extent,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: gpu_format(*format),
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                replacement.insert(name.clone(), Target { texture, view });
            }
        }
        let replacement_frame = self.frame.as_ref().map(|_| {
            crate::frame_v4::FrameGpu::new(
                device,
                new_size,
                self.params.is_some(),
                self.shadow.is_some(),
                self.history.is_some(),
                self.light,
            )
        });
        let replacement_history = self
            .history
            .as_ref()
            .map(|h| crate::history_v4::HistoryGpu::new(device, h.config.clone(), new_size));
        let validation = device.pop_error_scope().await;
        let oom = device.pop_error_scope().await;
        if let Some(error) = validation.or(oom) {
            return Err(format!(
                "v4 resize pack '{profile}': v4 GPU resize: {error}"
            ));
        }
        if !self
            .textures
            .keys()
            .all(|name| replacement.contains_key(name))
        {
            return Err(format!(
                "v4 resize pack '{profile}': v4 GPU resize left a target unallocated"
            ));
        }
        self.textures = replacement;
        self.frame = replacement_frame;
        self.lighting_phase = LightingPhase::Idle;
        self.history = replacement_history;
        if let Some(shadow) = &mut self.shadow {
            shadow.ready = false;
        }
        self.size = new_size;
        self.logical = logical;
        self.bytes = peak;
        self.budget = new_budget;
        self.device_limits = device_limits;
        self.generation += 1;
        Ok(())
    }

    pub async fn run(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        scene: &wgpu::TextureView,
    ) -> Result<V4Readback> {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("v4 frame"),
        });
        self.encode(device, &mut encoder, scene)?;

        let width = self.size[0];
        let height = self.size[1];
        let row_bytes = u64::from(width).checked_mul(4).ok_or("v4 row overflow")?;
        let padded_row = padded_row_bytes(width).ok_or("v4 row overflow")?;
        let final_view = &self
            .textures
            .get(&self.final_output)
            .ok_or_else(|| format!("v4 unknown final texture '{}'", self.final_output))?;
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("v4 final staging"),
            size: padded_row
                .checked_mul(u64::from(height))
                .ok_or("v4 staging overflow")?,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                texture: &final_view.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyBuffer {
                buffer: &staging,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_row as u32),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let mut buffer_staging: BTreeMap<String, wgpu::Buffer> = BTreeMap::new();
        for (name, buffer) in &self.buffers {
            let size = self.buffer_sizes[name];
            let target = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(name),
                size,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            encoder.copy_buffer_to_buffer(buffer, 0, &target, 0, size);
            buffer_staging.insert(name.clone(), target);
        }
        queue.submit([encoder.finish()]);

        let final_rgba = map_buffer(device, &staging).await?;
        let mut tight = Vec::with_capacity((row_bytes * u64::from(height)) as usize);
        for row in 0..u64::from(height) {
            let start = (row * padded_row) as usize;
            tight.extend_from_slice(&final_rgba[start..start + row_bytes as usize]);
        }
        staging.unmap();
        let mut buffers = BTreeMap::new();
        for (name, target) in &buffer_staging {
            buffers.insert(name.clone(), map_buffer(device, target).await?);
            target.unmap();
        }
        Ok(V4Readback {
            width,
            height,
            final_rgba: tight,
            buffers,
        })
    }

    pub fn stats(&self) -> serde_json::Value {
        serde_json::json!({
            "descriptor_version": 4,
            "profile": self.profile,
            "pass_order": self.passes.iter().map(|p| p.id.as_str()).collect::<Vec<_>>(),
            "textures": self.textures.len(),
            "buffers": self.buffers.len(),
            "programs": self.programs.len(),
            "resources": self.resources.len(),
            "logical_bytes": self.logical,
            "peak_bytes": self.bytes,
            "reserved_bytes": self.bytes,
            "budget_bytes": self.budget,
            "dimensions": self.size,
            "frames_run": self.frames_run,
            "history": self.history.as_ref().map(|history| history.stats()),
            "opaque_light_inputs": self.light,
            "authored_terrain_lighting": self.lighting.is_some(),
            "lighting_phase": format!("{:?}", self.lighting_phase).to_lowercase(),
            "opaque_light_reserved_bytes": if self.light { crate::frame_v4::light_reservation(self.size).unwrap_or(0) } else { 0 },
            "generation": self.generation,
            "device_limits": self.device_limits.to_json(),
        })
    }

    pub fn device_limits(&self) -> &V4DeviceLimitsReport {
        &self.device_limits
    }
}

async fn map_buffer(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Result<Vec<u8>> {
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    receiver
        .recv()
        .map_err(|_| "v4 readback channel closed".to_string())?
        .map_err(|e| format!("v4 readback map failed: {e}"))?;
    let data = slice.get_mapped_range().to_vec();
    Ok(data)
}
