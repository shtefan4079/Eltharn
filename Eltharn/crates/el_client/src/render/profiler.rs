use super::gpu_timer::{GPU_TIMED_PASS_COUNT, GpuTimedPass, GpuTimerStats, GpuTimingSample};
use crate::network::protocol_backend::IpcTelemetrySnapshot;
use std::collections::{HashMap, VecDeque};
use std::fmt::Write as _;
use std::time::Duration;
use web_time::Instant;
#[cfg(not(target_arch = "wasm32"))]
#[path = "benchmark_capture.rs"]
mod benchmark_capture;
const PROFILE_ENV: &str = "ELTHARN_RENDER_PROFILE";
const INTERVAL_ENV: &str = "ELTHARN_RENDER_PROFILE_INTERVAL_MS";
const THRESHOLD_ENV: &str = "ELTHARN_RENDER_PROFILE_THRESHOLD_MS";
const BUDGET_ENV: &str = "ELTHARN_RENDER_PROFILE_BUDGET_MS";
const MICROFREEZE_ENV: &str = "ELTHARN_RENDER_PROFILE_MICROFREEZE_MS";
const SESSION_SUMMARY_INTERVAL_ENV: &str = "ELTHARN_RENDER_PROFILE_SESSION_SUMMARY_MS";
const WORK_SPIKE_LOG_LIMIT_ENV: &str = "ELTHARN_RENDER_PROFILE_SPIKE_LOG_LIMIT";
const FRAME_HISTORY_LEN: usize = 120;
const MAX_PENDING_CLIENT_TICKS: usize = 16;
const DEFAULT_SESSION_SUMMARY_INTERVAL_MS: f64 = 10_000.0;
const DEFAULT_WORK_SPIKE_LOG_LIMIT: u64 = 3;
const SESSION_LATENCY_BIN_WIDTH_MS: f64 = 0.1;
const SESSION_LATENCY_BIN_COUNT: usize = 1_001;
const CADENCE_DISCONTINUITY_MS: f64 = 1_000.0;
fn print_profile_line(msg: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(msg));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if el_protocol_backend::diagnostics::console_enabled() {
            std::println!("{msg}");
        }
        el_protocol_backend::diagnostics::write_render_block(msg);
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct ScopeStats {
    calls: u64,
    total_ms: f64,
    max_ms: f64,
    last_ms: f64,
    file: &'static str,
    line: u32,
}
impl ScopeStats {
    fn record(&mut self, duration_ms: f64, file: &'static str, line: u32) {
        self.calls = self.calls.saturating_add(1);
        self.total_ms += duration_ms;
        self.max_ms = self.max_ms.max(duration_ms);
        self.last_ms = duration_ms;
        self.file = file;
        self.line = line;
    }
}
#[derive(Clone, Debug)]
struct SessionLatency {
    samples: u64,
    total_ms: f64,
    max_ms: f64,
    buckets: Box<[u64; SESSION_LATENCY_BIN_COUNT]>,
}
impl Default for SessionLatency {
    fn default() -> Self {
        Self {
            samples: 0,
            total_ms: 0.0,
            max_ms: 0.0,
            buckets: Box::new([0; SESSION_LATENCY_BIN_COUNT]),
        }
    }
}
impl SessionLatency {
    fn record(&mut self, duration_ms: f64) {
        let duration_ms = duration_ms.max(0.0);
        self.samples = self.samples.saturating_add(1);
        self.total_ms += duration_ms;
        self.max_ms = self.max_ms.max(duration_ms);
        let bucket = (duration_ms / SESSION_LATENCY_BIN_WIDTH_MS) as usize;
        let bucket = bucket.min(SESSION_LATENCY_BIN_COUNT - 1);
        self.buckets[bucket] = self.buckets[bucket].saturating_add(1);
    }
    fn average_ms(&self) -> f64 {
        self.total_ms / self.samples.max(1) as f64
    }
    fn percentile_ms(&self, percentile: f64) -> f64 {
        if self.samples == 0 {
            return 0.0;
        }
        let rank = (self.samples as f64 * percentile.clamp(0.0, 1.0))
            .ceil()
            .max(1.0) as u64;
        let mut seen = 0_u64;
        for (index, count) in self.buckets.iter().enumerate() {
            seen = seen.saturating_add(*count);
            if seen >= rank {
                return if index + 1 == SESSION_LATENCY_BIN_COUNT {
                    self.max_ms
                } else {
                    (index + 1) as f64 * SESSION_LATENCY_BIN_WIDTH_MS
                };
            }
        }
        self.max_ms
    }
}
#[derive(Clone, Debug, Default)]
struct GpuPassSummary {
    present_samples: u64,
    absent_samples: u64,
    duration: SessionLatency,
}
impl GpuPassSummary {
    fn record(&mut self, duration_ms: Option<f64>) {
        if let Some(duration_ms) = duration_ms {
            self.present_samples = self.present_samples.saturating_add(1);
            self.duration.record(duration_ms);
        } else {
            self.absent_samples = self.absent_samples.saturating_add(1);
        }
    }
    fn completed_samples(&self) -> u64 {
        self.present_samples.saturating_add(self.absent_samples)
    }
    fn presence_coverage_pct(&self) -> f64 {
        let completed_samples = self.completed_samples();
        if completed_samples == 0 {
            0.0
        } else {
            self.present_samples as f64 / completed_samples as f64 * 100.0
        }
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct SessionHostCpu {
    samples: u64,
    process_total_pct: f64,
    process_max_pct: f64,
    system_busy_total_pct: f64,
    system_busy_max_pct: f64,
    load_total: f64,
    load_max: f64,
}
impl SessionHostCpu {
    fn record(&mut self, load: CpuLoad) {
        self.samples = self.samples.saturating_add(1);
        self.process_total_pct += load.process_pct;
        self.process_max_pct = self.process_max_pct.max(load.process_pct);
        self.system_busy_total_pct += load.sys_busy_pct;
        self.system_busy_max_pct = self.system_busy_max_pct.max(load.sys_busy_pct);
        self.load_total += load.sys_load;
        self.load_max = self.load_max.max(load.sys_load);
    }
}
struct SessionSummary {
    started_at: Instant,
    started_unix_ms: u64,
    last_emitted_at: Instant,
    emit_interval: Duration,
    sequence: u64,
    last_emitted_frame_count: u64,
    scopes: HashMap<&'static str, ScopeStats>,
    mesher_worker: SessionLatency,
    work: SessionLatency,
    cadence: SessionLatency,
    cpu_work: SessionLatency,
    presentation_wait: SessionLatency,
    root_residual: SessionLatency,
    gpu_encoded_span: SessionLatency,
    gpu_pass_spans: [GpuPassSummary; GPU_TIMED_PASS_COUNT],
    gpu_samples_paired_with_cadence: u64,
    gpu_samples_unpaired_with_cadence: u64,
    profiler_interval_report_cpu: SessionLatency,
    profiler_detailed_spike_cpu: SessionLatency,
    profiler_summary_serialization_cpu: SessionLatency,
    work_spikes: u64,
    spike_logs_emitted: u64,
    spike_logs_suppressed: u64,
    cadence_stalls: u64,
    cpu_bound: u64,
    gpu_bound: u64,
    wait_bound: u64,
    mixed_bound: u64,
    timeline_frames: u64,
    timeline_cpu: SessionLatency,
    timeline_presentation: SessionLatency,
    timeline_pacer: SessionLatency,
    timeline_external: SessionLatency,
    throughput_work: u64,
    throughput_presentation: u64,
    throughput_cap: u64,
    throughput_external: u64,
    throughput_mixed: u64,
    pacer_reasons: HashMap<&'static str, u64>,
    pacer_caps: HashMap<u32, u64>,
    protocol_routes: HashMap<&'static str, u64>,
    ipc_telemetry: Option<IpcTelemetrySnapshot>,
    ipc_telemetry_observed_elapsed_ms: Option<f64>,
    remote_network_drain: RemoteNetworkDrainInterval,
    encode_counters: RenderEncodeCounters,
    encode_counter_max: RenderEncodeCounters,
    encode_counter_frames: u64,
    host_cpu: SessionHostCpu,
    gpu_memory_allocated_bytes_max: u64,
    gpu_memory_total_bytes: u64,
    gpu_memory_samples: u64,
}
impl SessionSummary {
    fn new(now: Instant, emit_interval: Duration) -> Self {
        Self {
            started_at: now,
            started_unix_ms: unix_timestamp_millis(),
            last_emitted_at: now,
            emit_interval,
            sequence: 0,
            last_emitted_frame_count: 0,
            scopes: HashMap::new(),
            mesher_worker: SessionLatency::default(),
            work: SessionLatency::default(),
            cadence: SessionLatency::default(),
            cpu_work: SessionLatency::default(),
            presentation_wait: SessionLatency::default(),
            root_residual: SessionLatency::default(),
            gpu_encoded_span: SessionLatency::default(),
            gpu_pass_spans: std::array::from_fn(|_| GpuPassSummary::default()),
            gpu_samples_paired_with_cadence: 0,
            gpu_samples_unpaired_with_cadence: 0,
            profiler_interval_report_cpu: SessionLatency::default(),
            profiler_detailed_spike_cpu: SessionLatency::default(),
            profiler_summary_serialization_cpu: SessionLatency::default(),
            work_spikes: 0,
            spike_logs_emitted: 0,
            spike_logs_suppressed: 0,
            cadence_stalls: 0,
            cpu_bound: 0,
            gpu_bound: 0,
            wait_bound: 0,
            mixed_bound: 0,
            timeline_frames: 0,
            timeline_cpu: SessionLatency::default(),
            timeline_presentation: SessionLatency::default(),
            timeline_pacer: SessionLatency::default(),
            timeline_external: SessionLatency::default(),
            throughput_work: 0,
            throughput_presentation: 0,
            throughput_cap: 0,
            throughput_external: 0,
            throughput_mixed: 0,
            pacer_reasons: HashMap::new(),
            pacer_caps: HashMap::new(),
            protocol_routes: HashMap::new(),
            ipc_telemetry: None,
            ipc_telemetry_observed_elapsed_ms: None,
            remote_network_drain: RemoteNetworkDrainInterval::default(),
            encode_counters: RenderEncodeCounters::default(),
            encode_counter_max: RenderEncodeCounters::default(),
            encode_counter_frames: 0,
            host_cpu: SessionHostCpu::default(),
            gpu_memory_allocated_bytes_max: 0,
            gpu_memory_total_bytes: 0,
            gpu_memory_samples: 0,
        }
    }
    fn due(&self, now: Instant) -> bool {
        self.work.samples > self.last_emitted_frame_count
            && now.duration_since(self.last_emitted_at) >= self.emit_interval
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RenderEncodeScopeCounters {
    render_passes: u32,
    draw_commands: u32,
    draw_calls: u32,
    pipeline_sets: u32,
    bind_group_sets: u32,
}
impl RenderEncodeScopeCounters {
    fn record_render_pass(&mut self) {
        self.render_passes = self.render_passes.saturating_add(1);
    }
    fn record_draw(&mut self, draw_calls: u32) {
        self.draw_commands = self.draw_commands.saturating_add(1);
        self.draw_calls = self.draw_calls.saturating_add(draw_calls);
    }
    fn record_pipeline_set(&mut self) {
        self.pipeline_sets = self.pipeline_sets.saturating_add(1);
    }
    fn record_bind_group_set(&mut self) {
        self.bind_group_sets = self.bind_group_sets.saturating_add(1);
    }
    fn accumulate(&mut self, other: Self) {
        self.render_passes = self.render_passes.saturating_add(other.render_passes);
        self.draw_commands = self.draw_commands.saturating_add(other.draw_commands);
        self.draw_calls = self.draw_calls.saturating_add(other.draw_calls);
        self.pipeline_sets = self.pipeline_sets.saturating_add(other.pipeline_sets);
        self.bind_group_sets = self.bind_group_sets.saturating_add(other.bind_group_sets);
    }
    fn maxima(&mut self, other: Self) {
        self.render_passes = self.render_passes.max(other.render_passes);
        self.draw_commands = self.draw_commands.max(other.draw_commands);
        self.draw_calls = self.draw_calls.max(other.draw_calls);
        self.pipeline_sets = self.pipeline_sets.max(other.pipeline_sets);
        self.bind_group_sets = self.bind_group_sets.max(other.bind_group_sets);
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RenderEncodeCounters {
    pub(crate) world: RenderEncodeScopeCounters,
    pub(crate) fallback_entities: RenderEncodeScopeCounters,
    pub(crate) local_player: RenderEncodeScopeCounters,
    pub(crate) name_tags: RenderEncodeScopeCounters,
}
impl RenderEncodeCounters {
    fn accumulate(&mut self, other: Self) {
        self.world.accumulate(other.world);
        self.fallback_entities.accumulate(other.fallback_entities);
        self.local_player.accumulate(other.local_player);
        self.name_tags.accumulate(other.name_tags);
    }
    fn maxima(&mut self, other: Self) {
        self.world.maxima(other.world);
        self.fallback_entities.maxima(other.fallback_entities);
        self.local_player.maxima(other.local_player);
        self.name_tags.maxima(other.name_tags);
    }
}
pub(crate) struct CountedRenderPass<'pass, 'counters> {
    pass: wgpu::RenderPass<'pass>,
    counters: Option<&'counters mut RenderEncodeScopeCounters>,
}
impl<'pass, 'counters> CountedRenderPass<'pass, 'counters> {
    pub(crate) fn new(
        pass: wgpu::RenderPass<'pass>,
        mut counters: Option<&'counters mut RenderEncodeScopeCounters>,
    ) -> Self {
        if let Some(counters) = counters.as_deref_mut() {
            counters.record_render_pass();
        }
        Self { pass, counters }
    }
    fn record(&mut self, record: impl FnOnce(&mut RenderEncodeScopeCounters)) {
        if let Some(counters) = self.counters.as_deref_mut() {
            record(counters);
        }
    }
    pub(crate) fn set_pipeline(&mut self, pipeline: &'pass wgpu::RenderPipeline) {
        self.record(RenderEncodeScopeCounters::record_pipeline_set);
        self.pass.set_pipeline(pipeline);
    }
    pub(crate) fn set_bind_group(
        &mut self,
        index: u32,
        bind_group: &'pass wgpu::BindGroup,
        offsets: &[wgpu::DynamicOffset],
    ) {
        self.record(RenderEncodeScopeCounters::record_bind_group_set);
        self.pass.set_bind_group(index, bind_group, offsets);
    }
    pub(crate) fn draw_indexed(
        &mut self,
        indices: std::ops::Range<u32>,
        base_vertex: i32,
        instances: std::ops::Range<u32>,
    ) {
        self.record(|counters| counters.record_draw(1));
        self.pass.draw_indexed(indices, base_vertex, instances);
    }
    pub(crate) fn multi_draw_indexed_indirect(
        &mut self,
        indirect_buffer: &'pass wgpu::Buffer,
        indirect_offset: wgpu::BufferAddress,
        count: u32,
    ) {
        self.record(|counters| counters.record_draw(count));
        self.pass
            .multi_draw_indexed_indirect(indirect_buffer, indirect_offset, count);
    }
}
impl<'pass, 'counters> std::ops::Deref for CountedRenderPass<'pass, 'counters> {
    type Target = wgpu::RenderPass<'pass>;
    fn deref(&self) -> &Self::Target {
        &self.pass
    }
}
impl<'pass, 'counters> std::ops::DerefMut for CountedRenderPass<'pass, 'counters> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pass
    }
}
impl<'a, 'pass, 'counters> crate::render::armor_pass::AvatarPassEncoder<'a>
    for CountedRenderPass<'pass, 'counters>
where
    'a: 'pass,
{
    fn avatar_set_pipeline(&mut self, pipeline: &'a wgpu::RenderPipeline) {
        self.set_pipeline(pipeline);
    }
    fn avatar_set_bind_group(
        &mut self,
        index: u32,
        bind_group: &'a wgpu::BindGroup,
        offsets: &[wgpu::DynamicOffset],
    ) {
        self.set_bind_group(index, bind_group, offsets);
    }
    fn avatar_set_vertex_buffer(&mut self, slot: u32, slice: wgpu::BufferSlice<'a>) {
        self.pass.set_vertex_buffer(slot, slice);
    }
    fn avatar_set_index_buffer(&mut self, slice: wgpu::BufferSlice<'a>, format: wgpu::IndexFormat) {
        self.pass.set_index_buffer(slice, format);
    }
    fn avatar_draw_indexed(
        &mut self,
        indices: std::ops::Range<u32>,
        base_vertex: i32,
        instances: std::ops::Range<u32>,
    ) {
        self.draw_indexed(indices, base_vertex, instances);
    }
}
#[derive(Clone, Copy, Debug, Default)]
struct FrameAccounting {
    update_ms: f64,
    render_ms: f64,
    surface_wait_ms: f64,
    present_ms: f64,
}
#[derive(Clone, Copy, Debug)]
struct PacerSample {
    duration: Duration,
    cap: u32,
    reason: &'static str,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RemoteNetworkDrainSample {
    processed_events: usize,
    queued_before: usize,
    queued_after: Option<usize>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct RemoteNetworkDrainInterval {
    drains: u64,
    processed_events: u64,
    max_processed_events: usize,
    queued_before_total: u64,
    queued_before_max: usize,
    queued_after_samples: u64,
    queued_after_total: u64,
    queued_after_max: usize,
    connection_closed_after_drain: u64,
}
impl RemoteNetworkDrainInterval {
    fn record(&mut self, sample: RemoteNetworkDrainSample) {
        self.drains = self.drains.saturating_add(1);
        self.processed_events = self
            .processed_events
            .saturating_add(sample.processed_events as u64);
        self.max_processed_events = self.max_processed_events.max(sample.processed_events);
        self.queued_before_total = self
            .queued_before_total
            .saturating_add(sample.queued_before as u64);
        self.queued_before_max = self.queued_before_max.max(sample.queued_before);
        if let Some(queued_after) = sample.queued_after {
            self.queued_after_samples = self.queued_after_samples.saturating_add(1);
            self.queued_after_total = self.queued_after_total.saturating_add(queued_after as u64);
            self.queued_after_max = self.queued_after_max.max(queued_after);
        } else {
            self.connection_closed_after_drain =
                self.connection_closed_after_drain.saturating_add(1);
        }
    }
    fn average_processed_events(self) -> f64 {
        self.processed_events as f64 / self.drains.max(1) as f64
    }
    fn average_queued_before(self) -> f64 {
        self.queued_before_total as f64 / self.drains.max(1) as f64
    }
    fn average_queued_after(self) -> f64 {
        self.queued_after_total as f64 / self.queued_after_samples.max(1) as f64
    }
}
struct RollingMedian {
    order: VecDeque<f64>,
    sorted: Vec<f64>,
    capacity: usize,
}
impl RollingMedian {
    fn new(capacity: usize) -> Self {
        Self {
            order: VecDeque::with_capacity(capacity),
            sorted: Vec::with_capacity(capacity),
            capacity,
        }
    }
    fn push(&mut self, value: f64) {
        if self.order.len() == self.capacity
            && let Some(expired) = self.order.pop_front()
        {
            let expired_index = self
                .sorted
                .binary_search_by(|candidate| candidate.total_cmp(&expired));
            debug_assert!(
                expired_index.is_ok(),
                "rolling median windows must contain identical samples"
            );
            match expired_index {
                Ok(index) => {
                    self.sorted.remove(index);
                }
                Err(_) => {
                    self.sorted.clear();
                    self.sorted.extend(self.order.iter().copied());
                    self.sorted.sort_by(f64::total_cmp);
                }
            }
        }
        let insertion = self
            .sorted
            .partition_point(|candidate| candidate.total_cmp(&value).is_lt());
        self.sorted.insert(insertion, value);
        self.order.push_back(value);
    }
    fn median_or(&self, fallback: f64) -> f64 {
        self.sorted
            .get(self.sorted.len() / 2)
            .copied()
            .unwrap_or(fallback)
    }
    fn len(&self) -> usize {
        self.order.len()
    }
    fn clear(&mut self) {
        self.order.clear();
        self.sorted.clear();
    }
}
#[derive(Clone, Copy, Debug)]
struct CpuLoad {
    process_pct: f64,
    sys_busy_pct: f64,
    sys_load: f64,
}
#[cfg(target_os = "linux")]
struct CpuSampler {
    prev_proc_ticks: u64,
    prev_sys_busy: u64,
    prev_sys_idle: u64,
    prev_wall: std::time::Instant,
    clk_tck: u64,
}
#[cfg(not(target_os = "linux"))]
struct CpuSampler;
#[cfg(target_os = "linux")]
fn read_proc_self_ticks() -> Option<u64> {
    let data = std::fs::read_to_string("/proc/self/stat").ok()?;
    let rparen = data.rfind(')')?;
    let mut fields = data[rparen + 1..].split_whitespace();
    let _state = fields.next();
    let utime: u64 = fields.nth(10)?.parse().ok()?;
    let stime: u64 = fields.next()?.parse().ok()?;
    Some(utime + stime)
}
#[cfg(target_os = "linux")]
fn read_proc_stat_cpu() -> Option<(u64, u64)> {
    let data = std::fs::read_to_string("/proc/stat").ok()?;
    let line = data.lines().next()?;
    let fields = line
        .split_whitespace()
        .skip(1)
        .filter_map(|value| value.parse::<u64>().ok())
        .collect::<Vec<_>>();
    if fields.len() < 5 {
        return None;
    }
    let busy = fields[0] + fields[1] + fields[2] + fields[5..].iter().sum::<u64>();
    let idle = fields[3] + fields[4];
    Some((busy, idle))
}
#[cfg(target_os = "linux")]
fn read_loadavg() -> f64 {
    std::fs::read_to_string("/proc/loadavg")
        .ok()
        .and_then(|data| data.split_whitespace().next().map(str::to_owned))
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(0.0)
}
impl CpuSampler {
    fn new() -> Self {
        #[cfg(target_os = "linux")]
        {
            let clk_tck = 100;
            let prev_proc_ticks = read_proc_self_ticks().unwrap_or(0);
            let (prev_sys_busy, prev_sys_idle) = read_proc_stat_cpu().unwrap_or((0, 0));
            Self {
                prev_proc_ticks,
                prev_sys_busy,
                prev_sys_idle,
                prev_wall: std::time::Instant::now(),
                clk_tck,
            }
        }
        #[cfg(not(target_os = "linux"))]
        {
            Self
        }
    }
    fn sample(&mut self) -> Option<CpuLoad> {
        #[cfg(target_os = "linux")]
        {
            let now = std::time::Instant::now();
            let proc_ticks = read_proc_self_ticks()?;
            let (sys_busy, sys_idle) = read_proc_stat_cpu()?;
            let wall_ms = now.duration_since(self.prev_wall).as_secs_f64() * 1000.0;
            if wall_ms <= 0.0 {
                return None;
            }
            let d_proc = proc_ticks.saturating_sub(self.prev_proc_ticks);
            let d_busy = sys_busy.saturating_sub(self.prev_sys_busy);
            let d_idle = sys_idle.saturating_sub(self.prev_sys_idle);
            self.prev_proc_ticks = proc_ticks;
            self.prev_sys_busy = sys_busy;
            self.prev_sys_idle = sys_idle;
            self.prev_wall = now;
            let process_pct = d_proc as f64 * (1000.0 / self.clk_tck as f64) / wall_ms * 100.0;
            let sys_total = (d_busy + d_idle).max(1) as f64;
            let sys_busy_pct = d_busy as f64 / sys_total * 100.0;
            Some(CpuLoad {
                process_pct,
                sys_busy_pct,
                sys_load: read_loadavg(),
            })
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = self;
            None
        }
    }
}
pub(crate) struct RenderProfiler {
    enabled: bool,
    client_tick_started_at: Vec<Instant>,
    client_tick_overflow_dropped: u64,
    client_tick_unpaired_dropped: u64,
    report_interval: Duration,
    slow_threshold_ms: f64,
    frame_budget_ms: f64,
    work_spike_delta_ms: f64,
    last_report_at: Instant,
    frame_id: u64,
    interval_frames: u64,
    interval_total_frame_ms: f64,
    interval_max_frame_ms: f64,
    interval_mesher_completed: u64,
    interval_mesher_total_us: u64,
    interval_mesher_max_us: u64,
    interval_cadence_total_ms: f64,
    interval_cadence_max_ms: f64,
    interval_cadence_frames: u64,
    interval_cadence_stalls: u64,
    interval_gpu_total_ms: f64,
    interval_gpu_paired_cadence_ms: f64,
    interval_gpu_frames: u64,
    interval_gpu_unpaired_samples: u64,
    interval_gpu_pass_spans: [GpuPassSummary; GPU_TIMED_PASS_COUNT],
    gpu_timer_available: Option<bool>,
    interval_gpu_timer_stats: GpuTimerStats,
    total_gpu_timer_stats: GpuTimerStats,
    interval_work_spikes: u64,
    interval_cpu_bound: u64,
    interval_gpu_bound: u64,
    interval_wait_bound: u64,
    interval_mixed_bound: u64,
    interval_timeline_frames: u64,
    interval_timeline_cpu_ms: f64,
    interval_timeline_presentation_ms: f64,
    interval_timeline_pacer_ms: f64,
    interval_timeline_external_ms: f64,
    interval_throughput_work: u64,
    interval_throughput_presentation: u64,
    interval_throughput_cap: u64,
    interval_throughput_external: u64,
    interval_throughput_mixed: u64,
    interval_pacer_reasons: HashMap<&'static str, u64>,
    interval_pacer_caps: HashMap<u32, u64>,
    interval_remote_network_drain: RemoteNetworkDrainInterval,
    interval_encode_counters: RenderEncodeCounters,
    interval_encode_counter_max: RenderEncodeCounters,
    interval_encode_counter_frames: u64,
    stats: HashMap<&'static str, ScopeStats>,
    session: SessionSummary,
    work_spike_log_limit: u64,
    interval_work_spike_logs_emitted: u64,
    interval_work_spike_logs_suppressed: u64,
    current_frame_scopes: Vec<(&'static str, f64, &'static str, u32)>,
    current_accounting: FrameAccounting,
    current_remote_network_drain: Option<RemoteNetworkDrainSample>,
    current_encode_counters: Option<RenderEncodeCounters>,
    frame_history: RollingMedian,
    cadence_history: RollingMedian,
    recent_frame_cadence: VecDeque<(u64, f64)>,
    last_work_ms: Option<f64>,
    last_presentation_wait_ms: f64,
    pending_pacer: Option<PacerSample>,
    last_work_spike: Option<(u64, f64, &'static str, &'static str)>,
    cpu_sampler: CpuSampler,
    #[cfg(not(target_arch = "wasm32"))]
    benchmark: Option<benchmark_capture::BenchmarkCollector>,
}
impl RenderProfiler {
    pub(crate) fn new() -> Self {
        let enabled = env_bool(PROFILE_ENV, false);
        let report_interval_ms = if cfg!(target_arch = "wasm32") {
            3_000.0
        } else {
            env_f64(INTERVAL_ENV, 1_000.0).max(100.0)
        };
        let slow_threshold_ms = env_f64(THRESHOLD_ENV, 0.05).max(0.001);
        let frame_budget_ms = if cfg!(target_arch = "wasm32") {
            25.0
        } else {
            env_f64(BUDGET_ENV, 16.67).max(0.01)
        };
        let work_spike_delta_ms = if cfg!(target_arch = "wasm32") {
            4.0
        } else {
            env_f64(MICROFREEZE_ENV, 2.0).max(0.001)
        };
        let session_summary_interval_ms = if cfg!(target_arch = "wasm32") {
            30_000.0
        } else {
            env_f64(
                SESSION_SUMMARY_INTERVAL_ENV,
                DEFAULT_SESSION_SUMMARY_INTERVAL_MS,
            )
            .max(1_000.0)
        };
        let work_spike_log_limit = if cfg!(target_arch = "wasm32") {
            4
        } else {
            env_u64(WORK_SPIKE_LOG_LIMIT_ENV, DEFAULT_WORK_SPIKE_LOG_LIMIT).min(1_024)
        };
        let now = Instant::now();
        if enabled {
            print_profile_line(&format!(
                "[RenderProfiler] enabled | Java zones: world -> sky/main/clouds/weather -> hand/screen-effects -> gui | reports={}ms | session-summary={}ms file=eltharn-performance-summary.jsonl | slow function >= {:.2}ms | frame budget={:.2}ms | work spike >= +{:.2}ms vs median detailed-limit={work_spike_log_limit}/interval",
                report_interval_ms as u64,
                session_summary_interval_ms as u64,
                slow_threshold_ms,
                frame_budget_ms,
                work_spike_delta_ms,
            ));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let benchmark = benchmark_capture::BenchmarkCollector::from_env_enabled(enabled);
        Self {
            enabled,
            client_tick_started_at: Vec::new(),
            client_tick_overflow_dropped: 0,
            client_tick_unpaired_dropped: 0,
            report_interval: Duration::from_secs_f64(report_interval_ms / 1_000.0),
            slow_threshold_ms,
            frame_budget_ms,
            work_spike_delta_ms,
            last_report_at: now,
            frame_id: 0,
            interval_frames: 0,
            interval_total_frame_ms: 0.0,
            interval_max_frame_ms: 0.0,
            interval_mesher_completed: 0,
            interval_mesher_total_us: 0,
            interval_mesher_max_us: 0,
            interval_cadence_total_ms: 0.0,
            interval_cadence_max_ms: 0.0,
            interval_cadence_frames: 0,
            interval_cadence_stalls: 0,
            interval_gpu_total_ms: 0.0,
            interval_gpu_paired_cadence_ms: 0.0,
            interval_gpu_frames: 0,
            interval_gpu_unpaired_samples: 0,
            interval_gpu_pass_spans: std::array::from_fn(|_| GpuPassSummary::default()),
            gpu_timer_available: None,
            interval_gpu_timer_stats: GpuTimerStats::default(),
            total_gpu_timer_stats: GpuTimerStats::default(),
            interval_work_spikes: 0,
            interval_cpu_bound: 0,
            interval_gpu_bound: 0,
            interval_wait_bound: 0,
            interval_mixed_bound: 0,
            interval_timeline_frames: 0,
            interval_timeline_cpu_ms: 0.0,
            interval_timeline_presentation_ms: 0.0,
            interval_timeline_pacer_ms: 0.0,
            interval_timeline_external_ms: 0.0,
            interval_throughput_work: 0,
            interval_throughput_presentation: 0,
            interval_throughput_cap: 0,
            interval_throughput_external: 0,
            interval_throughput_mixed: 0,
            interval_pacer_reasons: HashMap::new(),
            interval_pacer_caps: HashMap::new(),
            interval_remote_network_drain: RemoteNetworkDrainInterval::default(),
            interval_encode_counters: RenderEncodeCounters::default(),
            interval_encode_counter_max: RenderEncodeCounters::default(),
            interval_encode_counter_frames: 0,
            stats: HashMap::new(),
            session: SessionSummary::new(
                now,
                Duration::from_secs_f64(session_summary_interval_ms / 1_000.0),
            ),
            work_spike_log_limit,
            interval_work_spike_logs_emitted: 0,
            interval_work_spike_logs_suppressed: 0,
            current_frame_scopes: Vec::new(),
            current_accounting: FrameAccounting::default(),
            current_remote_network_drain: None,
            current_encode_counters: None,
            frame_history: RollingMedian::new(FRAME_HISTORY_LEN),
            cadence_history: RollingMedian::new(FRAME_HISTORY_LEN),
            recent_frame_cadence: VecDeque::with_capacity(16),
            last_work_ms: None,
            last_presentation_wait_ms: 0.0,
            pending_pacer: None,
            last_work_spike: None,
            cpu_sampler: CpuSampler::new(),
            #[cfg(not(target_arch = "wasm32"))]
            benchmark,
        }
    }
    pub(crate) fn begin_client_tick(&mut self) {
        if self.enabled {
            if self.client_tick_started_at.len() >= MAX_PENDING_CLIENT_TICKS {
                self.client_tick_started_at.remove(0);
                self.client_tick_overflow_dropped =
                    self.client_tick_overflow_dropped.saturating_add(1);
            }
            self.client_tick_started_at.push(Instant::now());
        }
    }
    pub(crate) fn finish_client_tick(&mut self) {
        if let Some(started) = self.client_tick_started_at.pop() {
            self.observe("ClientSession::logical_tick", started.elapsed());
        }
    }
    #[track_caller]
    pub(crate) fn begin_frame(&mut self, cadence: Duration) {
        if !self.enabled {
            return;
        }
        if !self.client_tick_started_at.is_empty() {
            self.client_tick_unpaired_dropped = self
                .client_tick_unpaired_dropped
                .saturating_add(self.client_tick_started_at.len() as u64);
            self.client_tick_started_at.clear();
        }
        let previous_pacer = self.pending_pacer.take();
        self.frame_id = self.frame_id.wrapping_add(1);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(benchmark) = self.benchmark.as_mut() {
            benchmark.on_begin_frame(self.frame_id, Instant::now());
        }
        let cadence_ms = cadence.as_secs_f64() * 1_000.0;
        let cadence_recordable = cadence_is_recordable(self.frame_id, cadence_ms);
        if !cadence_recordable && self.frame_id == 1 {
            self.last_report_at = Instant::now();
            self.cpu_sampler = CpuSampler::new();
        } else if !cadence_recordable {
            print_profile_line(&format!(
                "[RenderProfiler][CADENCE_DISCONTINUITY] before_frame={} gap={cadence_ms:.2}ms excluded_from_statistics=true",
                self.frame_id,
            ));
            self.cadence_history.clear();
            self.recent_frame_cadence.clear();
            self.last_work_ms = None;
            self.last_presentation_wait_ms = 0.0;
            self.last_report_at = Instant::now();
            self.cpu_sampler = CpuSampler::new();
        } else {
            let location = std::panic::Location::caller();
            self.observe_duration(
                "Frame::cadence",
                location.file(),
                location.line(),
                cadence,
                false,
            );
            self.interval_cadence_total_ms += cadence_ms;
            self.interval_cadence_max_ms = self.interval_cadence_max_ms.max(cadence_ms);
            self.interval_cadence_frames = self.interval_cadence_frames.saturating_add(1);
            self.session.cadence.record(cadence_ms);
            self.recent_frame_cadence
                .push_back((self.frame_id - 1, cadence_ms));
            while self.recent_frame_cadence.len() > 16 {
                self.recent_frame_cadence.pop_front();
            }
            self.record_previous_timeline(cadence_ms, previous_pacer);
            let cadence_baseline_ms = self.cadence_history.median_or(self.frame_budget_ms);
            if cadence_is_stall(
                cadence_ms,
                cadence_baseline_ms,
                self.cadence_history.len(),
                self.work_spike_delta_ms,
            ) {
                self.interval_cadence_stalls = self.interval_cadence_stalls.saturating_add(1);
                self.session.cadence_stalls = self.session.cadence_stalls.saturating_add(1);
                let previous_work_ms = self.last_work_ms.unwrap_or(0.0);
                print_profile_line(&format!(
                    "[RenderProfiler][CADENCE_STALL] before_frame={} start_to_start={cadence_ms:.2}ms ({:.1} FPS) baseline={cadence_baseline_ms:.2}ms previous_work={previous_work_ms:.2}ms outside_work={:.2}ms",
                    self.frame_id,
                    1_000.0 / cadence_ms.max(0.001),
                    (cadence_ms - previous_work_ms).max(0.0),
                ));
            }
            self.cadence_history.push(cadence_ms);
        }
        self.current_frame_scopes.clear();
        self.current_accounting = FrameAccounting::default();
        self.current_remote_network_drain = None;
        self.current_encode_counters = None;
    }
    #[inline(always)]
    pub(crate) fn frame_id(&self) -> u64 {
        self.frame_id
    }
    #[inline(always)]
    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }
    #[inline(always)]
    pub(crate) fn record_protocol_route(&mut self, route: &'static str) {
        if self.enabled {
            *self.session.protocol_routes.entry(route).or_default() += 1;
        }
    }
    #[inline(always)]
    pub(crate) fn external_telemetry_snapshot_due(&self) -> bool {
        self.enabled && self.session.due(Instant::now())
    }
    #[inline(always)]
    pub(crate) fn record_ipc_telemetry(&mut self, snapshot: IpcTelemetrySnapshot) {
        if !self.enabled {
            return;
        }
        self.session.ipc_telemetry = Some(snapshot);
        self.session.ipc_telemetry_observed_elapsed_ms = Some(
            Instant::now()
                .duration_since(self.session.started_at)
                .as_secs_f64()
                * 1_000.0,
        );
    }
    #[inline(always)]
    pub(crate) fn record_remote_network_drain(
        &mut self,
        processed_events: usize,
        queued_before: usize,
        queued_after: Option<usize>,
    ) {
        if !self.enabled {
            return;
        }
        let sample = RemoteNetworkDrainSample {
            processed_events,
            queued_before,
            queued_after,
        };
        self.current_remote_network_drain = Some(sample);
        self.interval_remote_network_drain.record(sample);
        self.session.remote_network_drain.record(sample);
    }
    #[inline(always)]
    pub(crate) fn record_encode_counters(&mut self, counters: RenderEncodeCounters) {
        if !self.enabled {
            return;
        }
        self.current_encode_counters = Some(counters);
        self.interval_encode_counters.accumulate(counters);
        self.interval_encode_counter_max.maxima(counters);
        self.interval_encode_counter_frames = self.interval_encode_counter_frames.saturating_add(1);
        self.session.encode_counters.accumulate(counters);
        self.session.encode_counter_max.maxima(counters);
        self.session.encode_counter_frames = self.session.encode_counter_frames.saturating_add(1);
    }
    #[inline(always)]
    pub(crate) fn record_gpu_memory(&mut self, allocated_bytes: u64, total_bytes: u64) {
        if !self.enabled {
            return;
        }
        self.session.gpu_memory_allocated_bytes_max = self
            .session
            .gpu_memory_allocated_bytes_max
            .max(allocated_bytes);
        self.session.gpu_memory_total_bytes = total_bytes;
        self.session.gpu_memory_samples = self.session.gpu_memory_samples.saturating_add(1);
    }
    #[inline(always)]
    pub(crate) fn record_gpu_timer_stats(&mut self, stats: Option<GpuTimerStats>) {
        if !self.enabled {
            return;
        }
        if let Some(stats) = stats {
            self.gpu_timer_available = Some(true);
            self.interval_gpu_timer_stats.merge(stats);
            self.total_gpu_timer_stats.merge(stats);
        } else {
            self.gpu_timer_available = Some(false);
        }
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    #[inline(always)]
    pub(crate) fn record_frame_pacing(
        &mut self,
        duration: Duration,
        cap: u32,
        reason: &'static str,
    ) {
        if self.enabled {
            self.pending_pacer = Some(PacerSample {
                duration,
                cap,
                reason,
            });
        }
    }
    fn record_previous_timeline(&mut self, cadence_ms: f64, pacer: Option<PacerSample>) {
        let (Some(work_ms), Some(pacer)) = (self.last_work_ms, pacer) else {
            return;
        };
        let available_after_work_ms = (cadence_ms - work_ms).max(0.0);
        let pacer_ms = (pacer.duration.as_secs_f64() * 1_000.0).min(available_after_work_ms);
        let external_ms = (available_after_work_ms - pacer_ms).max(0.0);
        let presentation_ms = self.last_presentation_wait_ms.min(work_ms);
        let cpu_ms = (work_ms - presentation_ms).max(0.0);
        self.interval_timeline_frames = self.interval_timeline_frames.saturating_add(1);
        self.interval_timeline_cpu_ms += cpu_ms;
        self.interval_timeline_presentation_ms += presentation_ms;
        self.interval_timeline_pacer_ms += pacer_ms;
        self.interval_timeline_external_ms += external_ms;
        self.session.timeline_frames = self.session.timeline_frames.saturating_add(1);
        self.session.timeline_cpu.record(cpu_ms);
        self.session.timeline_presentation.record(presentation_ms);
        self.session.timeline_pacer.record(pacer_ms);
        self.session.timeline_external.record(external_ms);
        *self.interval_pacer_reasons.entry(pacer.reason).or_default() += 1;
        *self.interval_pacer_caps.entry(pacer.cap).or_default() += 1;
        *self.session.pacer_reasons.entry(pacer.reason).or_default() += 1;
        *self.session.pacer_caps.entry(pacer.cap).or_default() += 1;
        match classify_frame_timeline(cadence_ms, cpu_ms, presentation_ms, pacer_ms, external_ms) {
            "work-bound" => {
                self.interval_throughput_work = self.interval_throughput_work.saturating_add(1);
                self.session.throughput_work = self.session.throughput_work.saturating_add(1);
            }
            "presentation-bound" => {
                self.interval_throughput_presentation =
                    self.interval_throughput_presentation.saturating_add(1);
                self.session.throughput_presentation =
                    self.session.throughput_presentation.saturating_add(1);
            }
            "cap-bound" => {
                self.interval_throughput_cap = self.interval_throughput_cap.saturating_add(1);
                self.session.throughput_cap = self.session.throughput_cap.saturating_add(1);
            }
            "external/event-bound" => {
                self.interval_throughput_external =
                    self.interval_throughput_external.saturating_add(1);
                self.session.throughput_external =
                    self.session.throughput_external.saturating_add(1);
            }
            _ => {
                self.interval_throughput_mixed = self.interval_throughput_mixed.saturating_add(1);
                self.session.throughput_mixed = self.session.throughput_mixed.saturating_add(1);
            }
        }
        self.observe_duration(
            "Frame::pacer",
            file!(),
            line!(),
            Duration::from_secs_f64(pacer_ms / 1_000.0),
            false,
        );
        self.observe_duration(
            "Frame::external_or_event",
            file!(),
            line!(),
            Duration::from_secs_f64(external_ms / 1_000.0),
            false,
        );
    }
    #[inline(always)]
    #[track_caller]
    pub(crate) fn observe(&mut self, name: &'static str, duration: Duration) {
        if self.enabled {
            let location = std::panic::Location::caller();
            self.observe_duration(name, location.file(), location.line(), duration, true);
        }
    }
    pub(crate) fn record_mesher_stats_from_atomics(&mut self) {
        if !self.enabled {
            return;
        }
        use crate::render::mesher::MESHER_STATS;
        use std::sync::atomic::Ordering;
        let completed = MESHER_STATS.completed.swap(0, Ordering::Relaxed);
        let total_us = MESHER_STATS.total_us.swap(0, Ordering::Relaxed);
        let max_us = MESHER_STATS.max_us.swap(0, Ordering::Relaxed);
        if completed == 0 {
            return;
        }
        let avg_ms = (total_us as f64 / completed as f64) / 1_000.0;
        for _ in 0..completed {
            self.session.mesher_worker.record(avg_ms);
        }
        self.interval_mesher_completed = self.interval_mesher_completed.saturating_add(completed);
        self.interval_mesher_total_us = self.interval_mesher_total_us.saturating_add(total_us);
        self.interval_mesher_max_us = self.interval_mesher_max_us.max(max_us);
    }
    pub(crate) fn end_frame(
        &mut self,
        frame_duration: Duration,
        gpu_sample: Option<GpuTimingSample>,
    ) {
        if !self.enabled {
            return;
        }
        self.record_mesher_stats_from_atomics();
        let location = std::panic::Location::caller();
        self.observe_duration(
            "Frame::total",
            location.file(),
            location.line(),
            frame_duration,
            true,
        );
        let now = Instant::now();
        let frame_ms = frame_duration.as_secs_f64() * 1_000.0;
        self.interval_frames = self.interval_frames.saturating_add(1);
        self.interval_total_frame_ms += frame_ms;
        self.interval_max_frame_ms = self.interval_max_frame_ms.max(frame_ms);
        self.session.work.record(frame_ms);
        if let Some(sample) = gpu_sample {
            self.session.gpu_encoded_span.record(sample.duration_ms);
            for pass in GpuTimedPass::ALL {
                let duration_ms = sample.pass_duration_ms(pass);
                self.interval_gpu_pass_spans[pass.index()].record(duration_ms);
                self.session.gpu_pass_spans[pass.index()].record(duration_ms);
            }
            if let Some((_, paired_cadence_ms)) = self
                .recent_frame_cadence
                .iter()
                .find(|(frame_id, _)| *frame_id == sample.frame_id)
            {
                self.interval_gpu_total_ms += sample.duration_ms;
                self.interval_gpu_frames = self.interval_gpu_frames.saturating_add(1);
                self.interval_gpu_paired_cadence_ms += *paired_cadence_ms;
                self.session.gpu_samples_paired_with_cadence = self
                    .session
                    .gpu_samples_paired_with_cadence
                    .saturating_add(1);
                if gpu_dominates_cadence(sample.duration_ms, *paired_cadence_ms) {
                    self.interval_gpu_bound = self.interval_gpu_bound.saturating_add(1);
                    self.session.gpu_bound = self.session.gpu_bound.saturating_add(1);
                }
            } else {
                self.interval_gpu_unpaired_samples =
                    self.interval_gpu_unpaired_samples.saturating_add(1);
                self.session.gpu_samples_unpaired_with_cadence = self
                    .session
                    .gpu_samples_unpaired_with_cadence
                    .saturating_add(1);
            }
        }
        self.last_work_ms = Some(frame_ms);
        self.frame_history.push(frame_ms);
        let baseline_ms = self.baseline_frame_ms();
        let update_ms = self.current_accounting.update_ms;
        let render_ms = self.current_accounting.render_ms;
        let wait_ms = self.current_accounting.surface_wait_ms + self.current_accounting.present_ms;
        self.last_presentation_wait_ms = wait_ms.min(frame_ms);
        let cpu_ms = (update_ms + render_ms - wait_ms).max(0.0);
        let accounted_ms = (update_ms + render_ms).min(frame_ms);
        let uncounted_ms = (frame_ms - accounted_ms).max(0.0);
        self.session.cpu_work.record(cpu_ms);
        self.session.presentation_wait.record(wait_ms);
        self.session.root_residual.record(uncounted_ms);
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(benchmark) = self.benchmark.as_mut() {
            benchmark.on_end_frame(
                self.frame_id,
                benchmark_capture::FrameWork {
                    work_ms: frame_ms,
                    cpu_work_ms: cpu_ms,
                    present_wait_ms: wait_ms,
                    residual_ms: uncounted_ms,
                },
                gpu_sample.map(|sample| (sample.frame_id, sample.duration_ms)),
                Instant::now(),
            );
        }
        let (bottleneck, hotspot) = self.classify(frame_ms, cpu_ms, wait_ms + uncounted_ms);
        self.tally_bottleneck(bottleneck);
        match bottleneck {
            "CPU-bound" => {
                self.session.cpu_bound = self.session.cpu_bound.saturating_add(1);
            }
            "wait/vsync-bound" => {
                self.session.wait_bound = self.session.wait_bound.saturating_add(1);
            }
            _ => self.session.mixed_bound = self.session.mixed_bound.saturating_add(1),
        }
        let over_budget = frame_ms >= self.frame_budget_ms;
        let relative_spike =
            self.frame_history.len() >= 30 && frame_ms >= baseline_ms + self.work_spike_delta_ms;
        if over_budget || relative_spike {
            self.interval_work_spikes = self.interval_work_spikes.saturating_add(1);
            self.session.work_spikes = self.session.work_spikes.saturating_add(1);
            if self.interval_work_spike_logs_emitted >= self.work_spike_log_limit {
                self.interval_work_spike_logs_suppressed =
                    self.interval_work_spike_logs_suppressed.saturating_add(1);
                self.session.spike_logs_suppressed =
                    self.session.spike_logs_suppressed.saturating_add(1);
            } else {
                let spike_report_started_at = Instant::now();
                let previous_work_spike = self.last_work_spike;
                let repeat = previous_work_spike.as_ref().is_some_and(
                    |&(_, prev_ms, prev_bottleneck, prev_hotspot)| {
                        prev_bottleneck == bottleneck
                            && prev_hotspot == hotspot
                            && (frame_ms - prev_ms).abs() <= 1.0
                    },
                );
                let mut report = format!(
                    "[RenderProfiler][WORK_SPIKE] frame={} work={:.2}ms ({:.1} FPS) baseline={:.2}ms delta={:+.2}ms cpu={:.2}ms gpu_sample={} uncounted={:.2}ms bottleneck={} hotspot={}",
                    self.frame_id,
                    frame_ms,
                    1_000.0 / frame_ms.max(0.001),
                    baseline_ms,
                    frame_ms - baseline_ms,
                    cpu_ms,
                    gpu_sample
                        .map(|sample| format!(
                            "{:.2}ms(frame={})",
                            sample.duration_ms, sample.frame_id
                        ))
                        .unwrap_or_else(|| "n/a".to_string()),
                    uncounted_ms,
                    bottleneck,
                    hotspot,
                );
                if let Some(network) = self.current_remote_network_drain {
                    let queued_after = network
                        .queued_after
                        .map(|count| count.to_string())
                        .unwrap_or_else(|| "connection-closed".to_string());
                    let _ = write!(
                        report,
                        " remote_events={} remote_queue_snapshot={}->{}",
                        network.processed_events, network.queued_before, queued_after,
                    );
                }
                if let Some(counters) = self.current_encode_counters {
                    report
                        .push_str(
                            "\n  ↳ direct encoder commands (pass/draw-api/draws/pipeline-set/bind-set): ",
                        );
                    append_encode_counters(&mut report, counters);
                }
                if repeat {
                    let _ = write!(
                        report,
                        " (repeat of frame {}: same bottleneck/hotspot, scope list unchanged)",
                        previous_work_spike.map(|(id, _, _, _)| id).unwrap_or(0)
                    );
                } else {
                    let mut scopes = self.current_frame_scopes.clone();
                    scopes.sort_by(|left, right| right.1.total_cmp(&left.1));
                    for (name, duration_ms, file, line) in scopes.iter().take(12) {
                        let _ = write!(
                            report,
                            "\n  ↳ {name:<40} {duration_ms:>8.2} ms   @ {file}:{line}"
                        );
                    }
                    if scopes.len() > 12 {
                        let _ = write!(
                            report,
                            "\n  ↳ ... {} more measured functions",
                            scopes.len() - 12
                        );
                    }
                }
                print_profile_line(&report);
                self.interval_work_spike_logs_emitted =
                    self.interval_work_spike_logs_emitted.saturating_add(1);
                self.session.spike_logs_emitted = self.session.spike_logs_emitted.saturating_add(1);
                self.session
                    .profiler_detailed_spike_cpu
                    .record(spike_report_started_at.elapsed().as_secs_f64() * 1_000.0);
                self.last_work_spike = Some((self.frame_id, frame_ms, bottleneck, hotspot));
            }
        }
        if now.duration_since(self.last_report_at) >= self.report_interval {
            let interval_report_started_at = Instant::now();
            self.print_report();
            self.session
                .profiler_interval_report_cpu
                .record(interval_report_started_at.elapsed().as_secs_f64() * 1_000.0);
            self.last_report_at = now;
            self.interval_frames = 0;
            self.interval_total_frame_ms = 0.0;
            self.interval_max_frame_ms = 0.0;
            self.interval_mesher_completed = 0;
            self.interval_mesher_total_us = 0;
            self.interval_mesher_max_us = 0;
            self.interval_cadence_total_ms = 0.0;
            self.interval_cadence_max_ms = 0.0;
            self.interval_cadence_frames = 0;
            self.interval_cadence_stalls = 0;
            self.interval_gpu_total_ms = 0.0;
            self.interval_gpu_paired_cadence_ms = 0.0;
            self.interval_gpu_frames = 0;
            self.interval_gpu_unpaired_samples = 0;
            self.interval_gpu_pass_spans = std::array::from_fn(|_| GpuPassSummary::default());
            self.interval_gpu_timer_stats = GpuTimerStats::default();
            self.interval_work_spikes = 0;
            self.interval_cpu_bound = 0;
            self.interval_gpu_bound = 0;
            self.interval_wait_bound = 0;
            self.interval_mixed_bound = 0;
            self.interval_timeline_frames = 0;
            self.interval_timeline_cpu_ms = 0.0;
            self.interval_timeline_presentation_ms = 0.0;
            self.interval_timeline_pacer_ms = 0.0;
            self.interval_timeline_external_ms = 0.0;
            self.interval_throughput_work = 0;
            self.interval_throughput_presentation = 0;
            self.interval_throughput_cap = 0;
            self.interval_throughput_external = 0;
            self.interval_throughput_mixed = 0;
            self.interval_pacer_reasons.clear();
            self.interval_pacer_caps.clear();
            self.interval_remote_network_drain = RemoteNetworkDrainInterval::default();
            self.interval_encode_counters = RenderEncodeCounters::default();
            self.interval_encode_counter_max = RenderEncodeCounters::default();
            self.interval_encode_counter_frames = 0;
            self.interval_work_spike_logs_emitted = 0;
            self.interval_work_spike_logs_suppressed = 0;
            self.stats.clear();
        }
        if self.session.due(now) {
            self.emit_session_summary("periodic", now);
        }
    }
    fn observe_duration(
        &mut self,
        name: &'static str,
        file: &'static str,
        line: u32,
        duration: Duration,
        mark_slow: bool,
    ) {
        let duration_ms = duration.as_secs_f64() * 1_000.0;
        match name {
            "State::update" => self.current_accounting.update_ms = duration_ms,
            "State::render" => self.current_accounting.render_ms = duration_ms,
            "State::render::surface_acquire_wait" => {
                self.current_accounting.surface_wait_ms = duration_ms;
            }
            "State::render::present" => self.current_accounting.present_ms = duration_ms,
            _ => {}
        }
        let stats = self.stats.entry(name).or_default();
        stats.record(duration_ms, file, line);
        self.session
            .scopes
            .entry(name)
            .or_default()
            .record(duration_ms, file, line);
        if mark_slow && duration_ms >= self.slow_threshold_ms {
            self.current_frame_scopes
                .push((name, duration_ms, file, line));
        }
    }
    fn baseline_frame_ms(&self) -> f64 {
        self.frame_history.median_or(self.frame_budget_ms)
    }
    fn classify(&self, frame_ms: f64, cpu_ms: f64, wait_ms: f64) -> (&'static str, &'static str) {
        let hotspot = self
            .current_frame_scopes
            .iter()
            .filter(|(name, _, _, _)| !is_aggregate_scope(name))
            .max_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(name, _, _, _)| *name)
            .unwrap_or("(none)");
        (classify_frame_phase(frame_ms, cpu_ms, wait_ms), hotspot)
    }
    fn tally_bottleneck(&mut self, bottleneck: &'static str) {
        match bottleneck {
            "GPU-bound" => {
                self.interval_gpu_bound = self.interval_gpu_bound.saturating_add(1);
            }
            "CPU-bound" => {
                self.interval_cpu_bound = self.interval_cpu_bound.saturating_add(1);
            }
            "wait/vsync-bound" => {
                self.interval_wait_bound = self.interval_wait_bound.saturating_add(1);
            }
            _ => self.interval_mixed_bound = self.interval_mixed_bound.saturating_add(1),
        }
    }
    fn print_report(&mut self) {
        let mut entries = self
            .stats
            .iter()
            .map(|(name, stats)| (*name, *stats))
            .collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            right
                .1
                .total_ms
                .total_cmp(&left.1.total_ms)
                .then_with(|| right.1.max_ms.total_cmp(&left.1.max_ms))
        });
        let mut report = format!(
            "[RenderProfiler] {} frames | all measured functions | slow-frame threshold {:.3}ms | columns: calls / total / avg / max / last",
            self.interval_frames, self.slow_threshold_ms,
        );
        let average_frame_ms = if self.interval_frames > 0 {
            self.interval_total_frame_ms / self.interval_frames as f64
        } else {
            0.0
        };
        let average_cadence_ms = if self.interval_cadence_frames > 0 {
            self.interval_cadence_total_ms / self.interval_cadence_frames as f64
        } else {
            0.0
        };
        let _ = write!(
            report,
            "\n  work: avg {average_frame_ms:.2}ms max {:.2}ms spikes {} (detailed emitted={} suppressed={}) | cadence: avg {average_cadence_ms:.2}ms ({:.1} FPS) max {:.2}ms stalls {} | phase tally: cpu-bound={} wait-bound={} mixed={} | gpu-dominant-samples={}",
            self.interval_max_frame_ms,
            self.interval_work_spikes,
            self.interval_work_spike_logs_emitted,
            self.interval_work_spike_logs_suppressed,
            1_000.0 / average_cadence_ms.max(0.001),
            self.interval_cadence_max_ms,
            self.interval_cadence_stalls,
            self.interval_cpu_bound,
            self.interval_wait_bound,
            self.interval_mixed_bound,
            self.interval_gpu_bound,
        );
        if self.interval_mesher_completed > 0 {
            let avg_ms = (self.interval_mesher_total_us as f64
                / self.interval_mesher_completed as f64)
                / 1_000.0;
            let max_ms = self.interval_mesher_max_us as f64 / 1_000.0;
            let _ = write!(
                report,
                "\n  mesher workers: {} sections | avg {avg_ms:.3}ms/section | max {max_ms:.3}ms | semantics=worker-thread-wall-time-approximate",
                self.interval_mesher_completed,
            );
        }
        if self.interval_timeline_frames > 0 {
            let frames = self.interval_timeline_frames as f64;
            let mut reasons = self
                .interval_pacer_reasons
                .iter()
                .map(|(reason, count)| format!("{reason}={count}"))
                .collect::<Vec<_>>();
            reasons.sort_unstable();
            let mut caps = self
                .interval_pacer_caps
                .iter()
                .map(|(cap, count)| format!("{cap}fps={count}"))
                .collect::<Vec<_>>();
            caps.sort_unstable();
            let _ = write!(
                report,
                "\n  full timeline ({} paired): cpu/work {:.2}ms + presentation {:.2}ms + pacer {:.2}ms + event/OS {:.2}ms | throughput: cap={} work={} presentation={} external/event={} mixed={} | caps [{}] reasons [{}]",
                self.interval_timeline_frames,
                self.interval_timeline_cpu_ms / frames,
                self.interval_timeline_presentation_ms / frames,
                self.interval_timeline_pacer_ms / frames,
                self.interval_timeline_external_ms / frames,
                self.interval_throughput_cap,
                self.interval_throughput_work,
                self.interval_throughput_presentation,
                self.interval_throughput_external,
                self.interval_throughput_mixed,
                caps.join(", "),
                reasons.join(", "),
            );
        } else {
            report
                .push_str("\n  full timeline: pacer samples unavailable on this platform/session");
        }
        if self.interval_encode_counter_frames > 0 {
            let frames = self.interval_encode_counter_frames;
            let _ = write!(
                report,
                "\n  direct encoder commands avg/frame (pass/draw-api/draws/pipeline-set/bind-set; world excludes child renderers): "
            );
            append_average_encode_counters(&mut report, self.interval_encode_counters, frames);
            report.push_str(" | max/frame: ");
            append_encode_counters(&mut report, self.interval_encode_counter_max);
        }
        let network_drain = self.interval_remote_network_drain;
        if network_drain.drains > 0 {
            let queued_after = if network_drain.queued_after_samples > 0 {
                format!(
                    "avg {:.1} max {} over {} attached drains",
                    network_drain.average_queued_after(),
                    network_drain.queued_after_max,
                    network_drain.queued_after_samples,
                )
            } else {
                "no attached post-drain snapshot".to_string()
            };
            let _ = write!(
                report,
                "\n  remote network drain (live queue snapshots): drains={} events={} avg {:.1}/drain max {} | before avg {:.1} max {} | after {} | connection-closed-after={}",
                network_drain.drains,
                network_drain.processed_events,
                network_drain.average_processed_events(),
                network_drain.max_processed_events,
                network_drain.average_queued_before(),
                network_drain.queued_before_max,
                queued_after,
                network_drain.connection_closed_after_drain,
            );
        }
        if let Some(load) = self.cpu_sampler.sample() {
            self.session.host_cpu.record(load);
            let _ = write!(
                report,
                "\n  host: process-cpu {:.1}% (of 1 core) | system-busy {:.1}% | 1-min load {:.2}",
                load.process_pct, load.sys_busy_pct, load.sys_load,
            );
        } else {
            report.push_str("\n  host: cpu sampling unavailable on this platform");
        }
        if self.interval_gpu_frames > 0 && self.interval_gpu_paired_cadence_ms > 0.0 {
            let gpu_busy_pct =
                self.interval_gpu_total_ms / self.interval_gpu_paired_cadence_ms * 100.0;
            let _ = write!(
                report,
                "\n  gpu: busy {gpu_busy_pct:.1}% of originating cadence | avg {:.2}ms/sample over {} paired samples | unpaired={}",
                self.interval_gpu_total_ms / self.interval_gpu_frames as f64,
                self.interval_gpu_frames,
                self.interval_gpu_unpaired_samples,
            );
        } else if self.gpu_timer_available == Some(true) {
            let _ = write!(
                report,
                "\n  gpu: no completed cadence-paired sample in this interval | unpaired={}",
                self.interval_gpu_unpaired_samples,
            );
        }
        if self.session.gpu_memory_samples > 0 && self.session.gpu_memory_total_bytes > 0 {
            let alloc_mb = self.session.gpu_memory_allocated_bytes_max as f64 / (1024.0 * 1024.0);
            let total_mb = self.session.gpu_memory_total_bytes as f64 / (1024.0 * 1024.0);
            let _ = write!(
                report,
                "\n  gpu memory: allocated_max={alloc_mb:.1}MB / total={total_mb:.1}MB ({:.1}%)",
                alloc_mb / total_mb.max(0.001) * 100.0,
            );
        }
        if self.gpu_timer_available == Some(true) {
            let completed_samples =
                self.interval_gpu_pass_spans[GpuTimedPass::World.index()].completed_samples();
            if completed_samples > 0 {
                let _ = write!(
                    report,
                    "\n  gpu major command spans (completed samples={completed_samples}; present/absent/avg/max ms; sparse command-encoder boundaries): "
                );
                append_gpu_pass_summaries(&mut report, &self.interval_gpu_pass_spans);
            } else {
                report.push_str(
                    "\n  gpu major command spans: no completed timestamp readback in this interval",
                );
            }
        }
        match self.gpu_timer_available {
            Some(true) => {
                let interval = self.interval_gpu_timer_stats;
                let total = self.total_gpu_timer_stats;
                let _ = write!(
                    report,
                    "\n  gpu timer: every {} frames | interval off-cadence={} requested={} resolve-submitted={} readback-completed={} skipped-busy={} map-failed={} lag avg={:.1}f max={}f | lifetime coverage resolve={:.1}% readback={:.1}%",
                    total.sample_interval_frames.max(interval.sample_interval_frames),
                    interval.off_cadence, interval.requested, interval.resolve_submitted,
                    interval.readback_completed, interval.skipped_busy, interval
                    .map_failed, interval.average_lag_frames(), interval.lag_max_frames,
                    total.resolve_coverage_pct(), total.readback_coverage_pct(),
                );
            }
            Some(false) => {
                report
                    .push_str(
                        "\n  gpu timer: unavailable (adapter/device lacks TIMESTAMP_QUERY); no GPU busy time fabricated",
                    )
            }
            None => {
                report
                    .push_str(
                        "\n  gpu timer: renderer has not yet reported timestamp-query availability",
                    )
            }
        }
        if entries.is_empty() {
            report.push_str("\n  no measured functions were observed");
            print_profile_line(&report);
            return;
        }
        let _ = write!(
            report,
            "\n  {:<44} {:>6} {:>10} {:>9} {:>9} {:>9}  location",
            "function", "calls", "total", "avg", "max", "last"
        );
        let _ = write!(report, "\n  {}", "─".repeat(112));
        for (name, stats) in entries.iter().take(32) {
            let average_ms = stats.total_ms / stats.calls.max(1) as f64;
            let _ = write!(
                report,
                "\n  {name:<44} {:>6} {:>8.2}ms {:>7.2}ms {:>7.2}ms {:>7.2}ms  {}:{}",
                stats.calls,
                stats.total_ms,
                average_ms,
                stats.max_ms,
                stats.last_ms,
                stats.file,
                stats.line,
            );
        }
        if entries.len() > 32 {
            let _ = write!(
                report,
                "\n  ... {} more measured functions",
                entries.len() - 32
            );
        }
        print_profile_line(&report);
    }
    pub(crate) fn finish_session(&mut self, reason: &'static str) {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(benchmark) = self.benchmark.as_mut() {
            benchmark.on_finish_session();
        }
        if self.enabled && self.session.work.samples > 0 {
            self.emit_session_summary(reason, Instant::now());
        }
    }
    fn emit_session_summary(&mut self, reason: &'static str, now: Instant) {
        let summary_serialization_started_at = Instant::now();
        self.session.sequence = self.session.sequence.saturating_add(1);
        let elapsed_ms = now.duration_since(self.session.started_at).as_secs_f64() * 1_000.0;
        let mut scopes = self
            .session
            .scopes
            .iter()
            .map(|(name, stats)| {
                serde_json::json!(
                    { "name" : name, "calls" : stats.calls, "total_ms" : stats.total_ms,
                    "avg_ms" : stats.total_ms / stats.calls.max(1) as f64, "max_ms" :
                    stats.max_ms, "last_ms" : stats.last_ms, "source_file" : stats.file,
                    "source_line" : stats.line, "semantics" : "inclusive_observed", }
                )
            })
            .collect::<Vec<_>>();
        scopes.sort_by(|left, right| {
            left["name"]
                .as_str()
                .unwrap_or("")
                .cmp(right["name"].as_str().unwrap_or(""))
        });
        let summary = &self.session;
        let timer = self.total_gpu_timer_stats;
        let record = serde_json::json!(
            { "kind" : "performance_session", "record" : if reason == "periodic" {
            "interval" } else { "session_final" }, "reason" : reason, "role" : "client",
            "session_id" : format!("client-pid-{}-{}", std::process::id(), summary
            .started_unix_ms,), "sequence" : summary.sequence, "emitted_unix_ms" :
            unix_timestamp_millis(), "elapsed_ms" : elapsed_ms, "frames_observed" :
            summary.work.samples, "cumulative_since_session_start" : true, "profile" : {
            "enabled" : true, "slow_scope_threshold_ms" : self.slow_threshold_ms,
            "work_spike_budget_ms" : self.frame_budget_ms, "gpu_timer_available" : self
            .gpu_timer_available, "gpu_sample_interval_frames" : timer
            .sample_interval_frames, "protocol_routes_attached" : summary
            .protocol_routes, }, "frame_work" : { "frame_total" : session_latency_json(&
            summary.work), "cpu_work_excluding_known_wait" : session_latency_json(&
            summary.cpu_work), "known_surface_and_present_wait" : session_latency_json(&
            summary.presentation_wait), "root_residual_unaccounted" :
            session_latency_json(& summary.root_residual), "semantics" :
            "non_overlapping_roots_per_frame; residual=Frame::total-(State::update+State::render), clamped at zero",
            }, "cadence_timeline" : { "start_to_start" : session_latency_json(& summary
            .cadence), "paired_frames" : summary.timeline_frames, "cpu_work" :
            session_latency_json(& summary.timeline_cpu), "presentation_wait" :
            session_latency_json(& summary.timeline_presentation), "pacer_wait" :
            session_latency_json(& summary.timeline_pacer), "external_or_event" :
            session_latency_json(& summary.timeline_external),
            "throughput_classification" : { "work" : summary.throughput_work,
            "presentation" : summary.throughput_presentation, "cap" : summary
            .throughput_cap, "external_or_event" : summary.throughput_external, "mixed" :
            summary.throughput_mixed, }, "pacer_reasons" : summary.pacer_reasons,
            "pacer_caps" : summary.pacer_caps, "semantics" :
            "timeline values belong between neighbouring frame starts and must not be added to frame_work",
            }, "gpu_encoded_span_sampled" : { "duration" : session_latency_json(& summary
            .gpu_encoded_span), "samples_paired_with_cadence" : summary
            .gpu_samples_paired_with_cadence, "samples_unpaired_with_cadence" : summary
            .gpu_samples_unpaired_with_cadence, "major_command_spans" :
            gpu_pass_spans_json(& summary.gpu_pass_spans), "timer" : { "off_cadence" :
            timer.off_cadence, "requested" : timer.requested, "resolve_submitted" : timer
            .resolve_submitted, "readback_completed" : timer.readback_completed,
            "skipped_busy" : timer.skipped_busy, "map_failed" : timer.map_failed,
            "readback_lag_avg_frames" : timer.average_lag_frames(),
            "readback_lag_max_frames" : timer.lag_max_frames, "resolve_coverage_pct" :
            timer.resolve_coverage_pct(), "readback_coverage_pct" : timer
            .readback_coverage_pct(), }, "semantics" :
            "sparse timestamp-query span inside the encoded frame; it excludes CPU queue.submit/present/display wait and currently starts after staged terrain upload preparation",
            "major_command_span_semantics" :
            "each named span uses command-encoder timestamp boundaries immediately around its existing render-pass command region. It measures GPU execution between those boundaries, not CPU encode/setup or Queue::write_buffer work. present/absent counts use completed readbacks only; missing boundaries are absent, never reported as zero. Major spans overlap neither by design nor by guarantee across future renderer changes, so they must not be added to the total span. Uninstrumented child renderers and passes remain outside this partial breakdown.",
            }, "host_cpu_client_process_only" : { "samples" : summary.host_cpu.samples,
            "process_avg_pct_of_one_core" : summary.host_cpu.process_total_pct / summary
            .host_cpu.samples.max(1) as f64, "process_max_pct_of_one_core" : summary
            .host_cpu.process_max_pct, "system_busy_avg_pct" : summary.host_cpu
            .system_busy_total_pct / summary.host_cpu.samples.max(1) as f64,
            "system_busy_max_pct" : summary.host_cpu.system_busy_max_pct,
            "system_load_avg" : summary.host_cpu.load_total / summary.host_cpu.samples
            .max(1) as f64, "system_load_max" : summary.host_cpu.load_max, "semantics" :
            "client process only; external 1.21.11 backend is a different process and, when enabled, writes its own eltharn-azalea-backend-performance-summary.jsonl",
            }, "mesher_workers" : { "sampled_sections" : self.session.mesher_worker
            .samples, "semantics" :
            "approximate: worker thread wall time, avg per section recorded as individual samples; max is not distributed",
            }, "ipc_reader_thread_remote" : ipc_telemetry_json(summary.ipc_telemetry,
            summary.ipc_telemetry_observed_elapsed_ms,),
            "remote_network_drain_main_thread" : remote_network_drain_json(summary
            .remote_network_drain), "direct_encoder_commands_partial" :
            encode_counters_json(summary.encode_counters, summary.encode_counter_max,
            summary.encode_counter_frames,), "spikes" : { "work_spikes" : summary
            .work_spikes, "cadence_stalls" : summary.cadence_stalls, "cpu_bound_frames" :
            summary.cpu_bound, "wait_bound_frames" : summary.wait_bound, "mixed_frames" :
            summary.mixed_bound, "gpu_dominant_samples" : summary.gpu_bound,
            "detailed_spike_logs_emitted" : summary.spike_logs_emitted,
            "detailed_spike_logs_suppressed" : summary.spike_logs_suppressed, "semantics"
            :
            "all spikes are counted; detailed text examples are rate-limited to reduce profiler observer effect",
            }, "profiler_observer_cost" : { "interval_report_cpu" :
            session_latency_json(& summary.profiler_interval_report_cpu),
            "detailed_spike_format_and_enqueue_cpu" : session_latency_json(& summary
            .profiler_detailed_spike_cpu), "summary_serialization_and_enqueue_cpu" :
            session_latency_json(& summary.profiler_summary_serialization_cpu),
            "semantics" :
            "measured profiler-side CPU outside Frame::total; it can affect subsequent cadence and is reported instead of hidden",
            }, "scopes" : scopes, "scope_semantics" :
            "every listed scope is an inclusive observed CPU wall-time aggregate. Nested scope totals overlap and must never be summed or divided into frame shares.",
            "gpu_memory" : { "samples" : self.session.gpu_memory_samples,
            "allocated_bytes_max" : self.session.gpu_memory_allocated_bytes_max,
            "total_bytes" : self.session.gpu_memory_total_bytes, "semantics" :
            "best-effort wgpu global_memory_info; zero on Metal/WebGPU/OpenGL", },
            "coverage" : { "main_thread_frame_roots" : true, "external_backend_process" :
            false, "ipc_reader_thread" : summary.ipc_telemetry.is_some_and(| snapshot |
            snapshot.available), "mesher_worker_interval_totals" : self
            .interval_mesher_completed > 0 || self.session.mesher_worker.samples > 0,
            "mesher_worker_full_distribution" : false, "direct_encoder_counters_complete"
            : false, "gpu_major_command_spans_sampled" : self.gpu_timer_available ==
            Some(true), "gpu_major_command_spans_complete" : false, }, }
        );
        match serde_json::to_string(&record) {
            #[cfg(not(target_arch = "wasm32"))]
            Ok(line) => el_protocol_backend::diagnostics::write_performance_summary_line(&line),
            #[cfg(target_arch = "wasm32")]
            Ok(line) => web_sys::console::log_1(&wasm_bindgen::JsValue::from_str(&line)),
            Err(error) => print_profile_line(&format!(
                "[RenderProfiler][SESSION_SUMMARY_ERROR] reason={reason} error={error}"
            )),
        }
        self.session.last_emitted_at = now;
        self.session.last_emitted_frame_count = self.session.work.samples;
        self.session
            .profiler_summary_serialization_cpu
            .record(summary_serialization_started_at.elapsed().as_secs_f64() * 1_000.0);
        self.emit_latency_totals_text(reason, elapsed_ms, now);
    }
    fn emit_latency_totals_text(&self, reason: &'static str, elapsed_ms: f64, _now: Instant) {
        use std::fmt::Write as _;
        let frames = self.session.work.samples;
        if frames == 0 {
            return;
        }
        let mut scopes: Vec<(&'static str, &ScopeStats)> = self
            .session
            .scopes
            .iter()
            .map(|(name, stats)| (*name, stats))
            .collect();
        scopes.sort_by(|a, b| b.1.total_ms.total_cmp(&a.1.total_ms));
        let mut block = format!(
            "=== Cumulative latency totals ({reason}) | session={elapsed_ms:.0}ms ({:.1}s) | {frames} frames ===\n",
            elapsed_ms / 1_000.0,
        );
        let _ = write!(
            block,
            "  {:<44} {:>8} {:>12} {:>9} {:>9} {:>9}\n",
            "function", "calls", "total_ms", "avg_ms", "max_ms", "last_ms",
        );
        let _ = write!(block, "  {}\n", "─".repeat(100));
        for (name, stats) in scopes.iter().take(64) {
            let avg_ms = stats.total_ms / stats.calls.max(1) as f64;
            let _ = write!(
                block,
                "  {:<44} {:>8} {:>10.2}ms {:>7.3}ms {:>7.3}ms {:>7.3}ms\n",
                name, stats.calls, stats.total_ms, avg_ms, stats.max_ms, stats.last_ms,
            );
        }
        if scopes.len() > 64 {
            let _ = write!(block, "  ... {} more functions\n", scopes.len() - 64);
        }
        let _ = write!(block, "\n  --- GPU encoded span (sampled, sparse) ---\n");
        let gpu = &self.session.gpu_encoded_span;
        if gpu.samples > 0 {
            let _ = write!(
                block,
                "  gpu_encoded_span_total          samples={} total={:.2}ms avg={:.3}ms p95={:.3}ms max={:.3}ms\n",
                gpu.samples,
                gpu.total_ms,
                gpu.average_ms(),
                gpu.percentile_ms(0.95),
                gpu.max_ms,
            );
        }
        if self.session.gpu_memory_samples > 0 {
            let alloc_mb = self.session.gpu_memory_allocated_bytes_max as f64 / (1024.0 * 1024.0);
            let total_mb = self.session.gpu_memory_total_bytes as f64 / (1024.0 * 1024.0);
            let _ = write!(
                block,
                "  gpu_memory_allocated_max={alloc_mb:.1}MB  gpu_memory_total={total_mb:.1}MB\n",
            );
        }
        let host = &self.session.host_cpu;
        if host.samples > 0 {
            let _ = write!(
                block,
                "\n  --- Host CPU (sampled at report intervals) ---\n"
            );
            let _ = write!(
                block,
                "  process_cpu_avg={:.1}%  process_cpu_max={:.1}%  system_busy_avg={:.1}%  load_avg={:.2}\n",
                host.process_total_pct / host.samples as f64,
                host.process_max_pct,
                host.system_busy_total_pct / host.samples as f64,
                host.load_total / host.samples as f64,
            );
        }
        #[cfg(not(target_arch = "wasm32"))]
        el_protocol_backend::diagnostics::write_latency_totals_block(&block);
    }
}
fn session_latency_json(latency: &SessionLatency) -> serde_json::Value {
    serde_json::json!(
        { "samples" : latency.samples, "total_ms" : latency.total_ms, "avg_ms" : latency
        .average_ms(), "p95_estimated_ms" : latency.percentile_ms(0.95),
        "p99_estimated_ms" : latency.percentile_ms(0.99), "max_ms" : latency.max_ms,
        "percentile_method" : "fixed_0.1ms_buckets_to_100ms_then_exact_max_overflow", }
    )
}
fn ipc_timing_json(calls: u64, total_ns: u64, max_ns: u64) -> serde_json::Value {
    let total_ms = total_ns as f64 / 1_000_000.0;
    serde_json::json!(
        { "calls" : calls, "total_ms" : total_ms, "avg_ms" : total_ms / calls.max(1) as
        f64, "max_ms" : max_ns as f64 / 1_000_000.0, }
    )
}
fn ipc_telemetry_json(
    snapshot: Option<IpcTelemetrySnapshot>,
    observed_elapsed_ms: Option<f64>,
) -> serde_json::Value {
    let Some(snapshot) = snapshot else {
        return serde_json::json!(
            { "available" : false, "reason" : "no_connection_snapshot_captured",
            "semantics" :
            "no native external IPC worker snapshot was available for this client summary",
            }
        );
    };
    serde_json::json!(
        { "available" : snapshot.available, "route" : snapshot.route,
        "observed_session_elapsed_ms" : observed_elapsed_ms, "stdout_read" : { "bytes" :
        snapshot.stdout_bytes, "eof_count" : snapshot.stdout_read_eofs, "error_count" :
        snapshot.stdout_read_errors, "wait" : ipc_timing_json(snapshot.stdout_read_calls,
        snapshot.stdout_read_wait_ns, snapshot.stdout_read_wait_max_ns,), },
        "inbound_json_decode" : { "error_count" : snapshot.inbound_decode_errors,
        "duration" : ipc_timing_json(snapshot.inbound_decode_calls, snapshot
        .inbound_decode_ns, snapshot.inbound_decode_max_ns,), },
        "inbound_map_and_enqueue" : { "error_count" : snapshot
        .inbound_map_enqueue_errors, "duration" : ipc_timing_json(snapshot
        .inbound_map_enqueue_calls, snapshot.inbound_map_enqueue_ns, snapshot
        .inbound_map_enqueue_max_ns,), }, "stdin_json_encode" : { "bytes" : snapshot
        .stdin_encoded_bytes, "error_count" : snapshot.stdin_encode_errors, "duration" :
        ipc_timing_json(snapshot.stdin_encode_calls, snapshot.stdin_encode_ns, snapshot
        .stdin_encode_max_ns,), }, "stdin_write_and_flush" : { "attempted_bytes" :
        snapshot.stdin_write_attempted_bytes, "error_count" : snapshot
        .stdin_write_flush_errors, "duration" : ipc_timing_json(snapshot
        .stdin_write_flush_calls, snapshot.stdin_write_flush_ns, snapshot
        .stdin_write_flush_max_ns,), }, "semantics" :
        "cumulative client-side wall time around native external JSON-lines IPC operations; it excludes backend-process CPU, server/network latency, OS pipe queue age, and exact partial bytes delivered after a failed write",
        }
    )
}
fn gpu_pass_spans_json(spans: &[GpuPassSummary; GPU_TIMED_PASS_COUNT]) -> serde_json::Value {
    let mut object = serde_json::Map::with_capacity(GPU_TIMED_PASS_COUNT);
    for pass in GpuTimedPass::ALL {
        let span = &spans[pass.index()];
        object.insert(
            pass.name().to_string(),
            serde_json::json!(
                { "present_completed_samples" : span.present_samples,
                "absent_completed_samples" : span.absent_samples,
                "completed_samples_considered" : span.completed_samples(),
                "presence_coverage_pct_of_completed_samples" : span
                .presence_coverage_pct(), "duration" : session_latency_json(& span
                .duration), }
            ),
        );
    }
    serde_json::Value::Object(object)
}
fn append_gpu_pass_summaries(report: &mut String, spans: &[GpuPassSummary; GPU_TIMED_PASS_COUNT]) {
    for (index, pass) in GpuTimedPass::ALL.iter().copied().enumerate() {
        if index > 0 {
            report.push(' ');
        }
        let span = &spans[pass.index()];
        if span.present_samples == 0 {
            let _ = write!(
                report,
                "{}[{}/{} n/a]",
                pass.name(),
                span.present_samples,
                span.absent_samples,
            );
        } else {
            let _ = write!(
                report,
                "{}[{}/{} {:.2}/{:.2}]",
                pass.name(),
                span.present_samples,
                span.absent_samples,
                span.duration.average_ms(),
                span.duration.max_ms,
            );
        }
    }
}
fn remote_network_drain_json(drain: RemoteNetworkDrainInterval) -> serde_json::Value {
    serde_json::json!(
        { "drains" : drain.drains, "processed_events" : drain.processed_events,
        "avg_events_per_drain" : drain.average_processed_events(), "max_events_per_drain"
        : drain.max_processed_events, "queue_before_avg" : drain.average_queued_before(),
        "queue_before_max" : drain.queued_before_max, "queue_after_samples" : drain
        .queued_after_samples, "queue_after_avg" : drain.average_queued_after(),
        "queue_after_max" : drain.queued_after_max, "connection_closed_after_drain" :
        drain.connection_closed_after_drain, "semantics" :
        "receiver len snapshots are concurrent observations, not end-to-end packet latency",
        }
    )
}
fn encode_scope_json(counters: RenderEncodeScopeCounters) -> serde_json::Value {
    serde_json::json!(
        { "render_passes" : counters.render_passes, "draw_api_commands" : counters
        .draw_commands, "effective_gpu_draws" : counters.draw_calls, "pipeline_sets" :
        counters.pipeline_sets, "bind_group_sets" : counters.bind_group_sets, }
    )
}
fn encode_counters_json(
    total: RenderEncodeCounters,
    max: RenderEncodeCounters,
    frames: u64,
) -> serde_json::Value {
    serde_json::json!(
        { "frames" : frames, "total" : { "world" : encode_scope_json(total.world),
        "fallback_entities" : encode_scope_json(total.fallback_entities), "local_player"
        : encode_scope_json(total.local_player), "name_tags" : encode_scope_json(total
        .name_tags), }, "max_per_frame" : { "world" : encode_scope_json(max.world),
        "fallback_entities" : encode_scope_json(max.fallback_entities), "local_player" :
        encode_scope_json(max.local_player), "name_tags" : encode_scope_json(max
        .name_tags), }, "semantics" :
        "only State/ClientRenderer commands routed through CountedRenderPass; child renderers using a raw wgpu pass are explicitly excluded",
        }
    )
}
fn append_encode_scope(report: &mut String, label: &str, counters: RenderEncodeScopeCounters) {
    let _ = write!(
        report,
        "{label}[{}/{}/{}/{}/{}]",
        counters.render_passes,
        counters.draw_commands,
        counters.draw_calls,
        counters.pipeline_sets,
        counters.bind_group_sets,
    );
}
fn append_encode_counters(report: &mut String, counters: RenderEncodeCounters) {
    append_encode_scope(report, "world", counters.world);
    report.push(' ');
    append_encode_scope(report, "fallback", counters.fallback_entities);
    report.push(' ');
    append_encode_scope(report, "local", counters.local_player);
    report.push(' ');
    append_encode_scope(report, "name_tags", counters.name_tags);
}
fn append_average_encode_scope(
    report: &mut String,
    label: &str,
    counters: RenderEncodeScopeCounters,
    frames: u64,
) {
    let divisor = frames.max(1);
    let _ = write!(
        report,
        "{label}[{:.1}/{:.1}/{:.1}/{:.1}/{:.1}]",
        counters.render_passes as f64 / divisor as f64,
        counters.draw_commands as f64 / divisor as f64,
        counters.draw_calls as f64 / divisor as f64,
        counters.pipeline_sets as f64 / divisor as f64,
        counters.bind_group_sets as f64 / divisor as f64,
    );
}
fn append_average_encode_counters(
    report: &mut String,
    counters: RenderEncodeCounters,
    frames: u64,
) {
    append_average_encode_scope(report, "world", counters.world, frames);
    report.push(' ');
    append_average_encode_scope(report, "fallback", counters.fallback_entities, frames);
    report.push(' ');
    append_average_encode_scope(report, "local", counters.local_player, frames);
    report.push(' ');
    append_average_encode_scope(report, "name_tags", counters.name_tags, frames);
}
fn is_aggregate_scope(name: &str) -> bool {
    matches!(
        name,
        "Frame::total"
            | "State::update"
            | "State::render"
            | "State::render::encode_command_buffer"
            | "GameRenderer::renderLevel"
    )
}
fn classify_frame_phase(frame_ms: f64, cpu_ms: f64, wait_ms: f64) -> &'static str {
    let threshold = frame_ms * 0.75;
    if wait_ms >= threshold {
        "wait/vsync-bound"
    } else if cpu_ms >= threshold {
        "CPU-bound"
    } else {
        "mixed/unclassified"
    }
}
fn classify_frame_timeline(
    cadence_ms: f64,
    cpu_ms: f64,
    presentation_ms: f64,
    pacer_ms: f64,
    external_ms: f64,
) -> &'static str {
    let threshold = cadence_ms * 0.5;
    let components = [
        (cpu_ms, "work-bound"),
        (presentation_ms, "presentation-bound"),
        (pacer_ms, "cap-bound"),
        (external_ms, "external/event-bound"),
    ];
    let (largest_ms, label) = components
        .into_iter()
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .unwrap_or((0.0, "mixed/unclassified"));
    if largest_ms >= threshold {
        label
    } else {
        "mixed/unclassified"
    }
}
fn gpu_dominates_cadence(gpu_ms: f64, cadence_ms: f64) -> bool {
    gpu_ms >= cadence_ms * 0.75
}
fn cadence_is_discontinuity(cadence_ms: f64) -> bool {
    cadence_ms >= CADENCE_DISCONTINUITY_MS
}
fn cadence_is_recordable(frame_id: u64, cadence_ms: f64) -> bool {
    frame_id > 1 && !cadence_is_discontinuity(cadence_ms)
}
fn cadence_is_stall(
    cadence_ms: f64,
    baseline_ms: f64,
    history_len: usize,
    spike_delta_ms: f64,
) -> bool {
    history_len >= 30 && cadence_ms >= baseline_ms + spike_delta_ms
}
fn env_bool(name: &str, default: bool) -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        default
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        match std::env::var(name).ok().as_deref() {
            Some(value) if matches!(value, "0" | "false" | "FALSE" | "off" | "OFF") => false,
            Some(value) if matches!(value, "1" | "true" | "TRUE" | "on" | "ON") => true,
            _ => default,
        }
    }
}
fn env_f64(name: &str, default: f64) -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        default
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite())
            .unwrap_or(default)
    }
}
fn env_u64(name: &str, default: u64) -> u64 {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = name;
        default
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(name)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(default)
    }
}
fn unix_timestamp_millis() -> u64 {
    web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
