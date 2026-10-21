use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
const TOTAL_START_QUERY: u32 = 0;
const TOTAL_END_QUERY: u32 = 1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum GpuTimedPass {
    World,
    FallbackEntities,
    LocalPlayerAndHands,
    WorldNameTags,
    GuiAndInventoryPreview,
    Sky,
    Clouds,
    Weather,
    Particles,
}
pub(crate) const GPU_TIMED_PASS_COUNT: usize = 9;
impl GpuTimedPass {
    pub(crate) const ALL: [Self; GPU_TIMED_PASS_COUNT] = [
        Self::World,
        Self::FallbackEntities,
        Self::LocalPlayerAndHands,
        Self::WorldNameTags,
        Self::GuiAndInventoryPreview,
        Self::Sky,
        Self::Clouds,
        Self::Weather,
        Self::Particles,
    ];
    pub(crate) const fn index(self) -> usize {
        self as usize
    }
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::World => "world",
            Self::FallbackEntities => "fallback_entities",
            Self::LocalPlayerAndHands => "local_player_and_hands",
            Self::WorldNameTags => "world_name_tags",
            Self::GuiAndInventoryPreview => "gui_and_inventory_preview",
            Self::Sky => "sky",
            Self::Clouds => "clouds",
            Self::Weather => "weather",
            Self::Particles => "particles",
        }
    }
    const fn bit(self) -> u16 {
        1 << self.index()
    }
    const fn start_query(self) -> u32 {
        2 + self.index() as u32 * 2
    }
    const fn end_query(self) -> u32 {
        self.start_query() + 1
    }
}
const QUERY_COUNT: u32 = 2 + (GPU_TIMED_PASS_COUNT as u32 * 2);
const RESOLVE_SIZE: u64 = (QUERY_COUNT * 8) as u64;
#[cfg(not(target_arch = "wasm32"))]
const SAMPLE_INTERVAL_ENV: &str = "ELTHARN_RENDER_PROFILE_GPU_INTERVAL_FRAMES";
const DEFAULT_SAMPLE_INTERVAL_FRAMES: u32 = 8;
#[cfg(not(target_arch = "wasm32"))]
const MAX_SAMPLE_INTERVAL_FRAMES: u32 = 120;
const IDLE: u8 = 0;
const MAPPING: u8 = 1;
const DONE: u8 = 2;
const FAILED: u8 = 3;
struct GpuTimerReadback {
    buffer: wgpu::Buffer,
    state: Arc<AtomicU8>,
    frame_id: Option<u64>,
    pass_mask: u16,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct GpuTimingSample {
    pub(crate) frame_id: u64,
    pub(crate) duration_ms: f64,
    pub(crate) pass_durations_ms: [Option<f64>; GPU_TIMED_PASS_COUNT],
}
impl GpuTimingSample {
    pub(crate) fn pass_duration_ms(self, pass: GpuTimedPass) -> Option<f64> {
        self.pass_durations_ms[pass.index()]
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GpuTimerFramePlan {
    pub(crate) query_started: bool,
    pub(crate) poll_due: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct GpuTimerStats {
    pub(crate) sample_interval_frames: u32,
    pub(crate) off_cadence: u64,
    pub(crate) requested: u64,
    pub(crate) resolve_submitted: u64,
    pub(crate) readback_completed: u64,
    pub(crate) skipped_busy: u64,
    pub(crate) map_failed: u64,
    pub(crate) lag_total_frames: u64,
    pub(crate) lag_max_frames: u64,
}
impl GpuTimerStats {
    fn new(sample_interval_frames: u32) -> Self {
        Self {
            sample_interval_frames,
            ..Self::default()
        }
    }
    pub(crate) fn merge(&mut self, other: Self) {
        if other.sample_interval_frames != 0 {
            self.sample_interval_frames = other.sample_interval_frames;
        }
        self.off_cadence = self.off_cadence.saturating_add(other.off_cadence);
        self.requested = self.requested.saturating_add(other.requested);
        self.resolve_submitted = self
            .resolve_submitted
            .saturating_add(other.resolve_submitted);
        self.readback_completed = self
            .readback_completed
            .saturating_add(other.readback_completed);
        self.skipped_busy = self.skipped_busy.saturating_add(other.skipped_busy);
        self.map_failed = self.map_failed.saturating_add(other.map_failed);
        self.lag_total_frames = self.lag_total_frames.saturating_add(other.lag_total_frames);
        self.lag_max_frames = self.lag_max_frames.max(other.lag_max_frames);
    }
    pub(crate) fn resolve_coverage_pct(self) -> f64 {
        percentage(self.resolve_submitted, self.requested)
    }
    pub(crate) fn readback_coverage_pct(self) -> f64 {
        percentage(self.readback_completed, self.resolve_submitted)
    }
    pub(crate) fn average_lag_frames(self) -> f64 {
        if self.readback_completed == 0 {
            0.0
        } else {
            self.lag_total_frames as f64 / self.readback_completed as f64
        }
    }
}
pub(crate) struct GpuTimer {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readbacks: Vec<GpuTimerReadback>,
    next_readback: usize,
    timestamp_period_ns: f64,
    sample_interval_frames: u32,
    latest_frame_id: u64,
    active_readback: Option<usize>,
    active_pass_starts: u16,
    active_pass_ends: u16,
    completed_samples: VecDeque<GpuTimingSample>,
    interval_stats: GpuTimerStats,
}
impl GpuTimer {
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Option<Self> {
        if !device.features().contains(wgpu::Features::TIMESTAMP_QUERY) {
            return None;
        }
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("eltharn_gpu_timer_query_set"),
            ty: wgpu::QueryType::Timestamp,
            count: QUERY_COUNT,
        });
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("eltharn_gpu_timer_resolve"),
            size: RESOLVE_SIZE,
            usage: wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::QUERY_RESOLVE,
            mapped_at_creation: false,
        });
        let readbacks = (0..2)
            .map(|index| {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some(&format!("eltharn_gpu_timer_readback_{index}")),
                    size: RESOLVE_SIZE,
                    usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                GpuTimerReadback {
                    buffer,
                    state: Arc::new(AtomicU8::new(IDLE)),
                    frame_id: None,
                    pass_mask: 0,
                }
            })
            .collect();
        let sample_interval_frames = sample_interval_from_env();
        Some(Self {
            query_set,
            resolve_buffer,
            readbacks,
            next_readback: 0,
            timestamp_period_ns: f64::from(queue.get_timestamp_period()),
            sample_interval_frames,
            latest_frame_id: 0,
            active_readback: None,
            active_pass_starts: 0,
            active_pass_ends: 0,
            completed_samples: VecDeque::with_capacity(2),
            interval_stats: GpuTimerStats::new(sample_interval_frames),
        })
    }
    pub(crate) fn begin(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        frame_id: u64,
    ) -> GpuTimerFramePlan {
        self.latest_frame_id = frame_id;
        self.active_readback = None;
        self.active_pass_starts = 0;
        self.active_pass_ends = 0;
        if !frame_is_sampled(frame_id, self.sample_interval_frames) {
            self.interval_stats.off_cadence = self.interval_stats.off_cadence.saturating_add(1);
            return GpuTimerFramePlan::default();
        }
        self.interval_stats.requested = self.interval_stats.requested.saturating_add(1);
        self.collect_completed_readbacks();
        let Some(target_index) = self.available_readback_index() else {
            self.interval_stats.skipped_busy = self.interval_stats.skipped_busy.saturating_add(1);
            return GpuTimerFramePlan {
                query_started: false,
                poll_due: true,
            };
        };
        self.active_readback = Some(target_index);
        encoder.write_timestamp(&self.query_set, TOTAL_START_QUERY);
        GpuTimerFramePlan {
            query_started: true,
            poll_due: true,
        }
    }
    pub(crate) fn begin_pass(&mut self, encoder: &mut wgpu::CommandEncoder, pass: GpuTimedPass) {
        if self.active_readback.is_none() || self.active_pass_starts & pass.bit() != 0 {
            return;
        }
        encoder.write_timestamp(&self.query_set, pass.start_query());
        self.active_pass_starts |= pass.bit();
    }
    pub(crate) fn end_pass(&mut self, encoder: &mut wgpu::CommandEncoder, pass: GpuTimedPass) {
        if self.active_readback.is_none()
            || self.active_pass_starts & pass.bit() == 0
            || self.active_pass_ends & pass.bit() != 0
        {
            return;
        }
        encoder.write_timestamp(&self.query_set, pass.end_query());
        self.active_pass_ends |= pass.bit();
    }
    pub(crate) fn end(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let Some(target_index) = self.active_readback.take() else {
            return;
        };
        let target = &mut self.readbacks[target_index];
        encoder.write_timestamp(&self.query_set, TOTAL_END_QUERY);
        encoder.resolve_query_set(&self.query_set, 0..QUERY_COUNT, &self.resolve_buffer, 0);
        encoder.copy_buffer_to_buffer(&self.resolve_buffer, 0, &target.buffer, 0, RESOLVE_SIZE);
        target.frame_id = Some(self.latest_frame_id);
        target.pass_mask = self.active_pass_starts & self.active_pass_ends;
        self.interval_stats.resolve_submitted =
            self.interval_stats.resolve_submitted.saturating_add(1);
    }
    pub(crate) fn poll_readback(&mut self, device: &wgpu::Device) -> bool {
        self.collect_completed_readbacks();
        let start = self.next_readback;
        for offset in 0..self.readbacks.len() {
            let index = (start + offset) % self.readbacks.len();
            let readback = &self.readbacks[index];
            if readback.state.load(Ordering::SeqCst) != IDLE || readback.frame_id.is_none() {
                continue;
            }
            let state = readback.state.clone();
            let slice = readback.buffer.slice(..);
            readback.state.store(MAPPING, Ordering::SeqCst);
            slice.map_async(wgpu::MapMode::Read, move |result| {
                state.store(if result.is_ok() { DONE } else { FAILED }, Ordering::SeqCst);
            });
            self.next_readback = (index + 1) % self.readbacks.len();
        }
        let has_pending_mapping = self
            .readbacks
            .iter()
            .any(|readback| readback.state.load(Ordering::SeqCst) == MAPPING);
        if has_pending_mapping {
            device.poll(wgpu::Maintain::Poll);
        }
        has_pending_mapping
    }
    fn available_readback_index(&self) -> Option<usize> {
        (0..self.readbacks.len())
            .map(|offset| (self.next_readback + offset) % self.readbacks.len())
            .find(|&index| {
                let readback = &self.readbacks[index];
                readback.state.load(Ordering::SeqCst) == IDLE && readback.frame_id.is_none()
            })
    }
    fn collect_completed_readbacks(&mut self) {
        for readback in &mut self.readbacks {
            match readback.state.load(Ordering::SeqCst) {
                DONE => {
                    let frame_id = readback.frame_id.take();
                    let pass_mask = std::mem::take(&mut readback.pass_mask);
                    let range = readback.buffer.slice(..).get_mapped_range();
                    if range.len() >= RESOLVE_SIZE as usize {
                        let t0 = u64::from_le_bytes(
                            range[0..8].try_into().expect("timestamp start is 8 bytes"),
                        );
                        let t1 = u64::from_le_bytes(
                            range[8..16].try_into().expect("timestamp end is 8 bytes"),
                        );
                        if let Some(frame_id) = frame_id {
                            let mut pass_durations_ms = [None; GPU_TIMED_PASS_COUNT];
                            for pass in GpuTimedPass::ALL {
                                if pass_mask & pass.bit() == 0 {
                                    continue;
                                }
                                let start = pass.start_query() as usize * 8;
                                let end = pass.end_query() as usize * 8;
                                let t_start = u64::from_le_bytes(
                                    range[start..start + 8]
                                        .try_into()
                                        .expect("timestamp start is 8 bytes"),
                                );
                                let t_end = u64::from_le_bytes(
                                    range[end..end + 8]
                                        .try_into()
                                        .expect("timestamp end is 8 bytes"),
                                );
                                pass_durations_ms[pass.index()] = Some(timestamp_ticks_to_ms(
                                    t_end.saturating_sub(t_start),
                                    self.timestamp_period_ns,
                                ));
                            }
                            self.completed_samples.push_back(GpuTimingSample {
                                frame_id,
                                duration_ms: timestamp_ticks_to_ms(
                                    t1.saturating_sub(t0),
                                    self.timestamp_period_ns,
                                ),
                                pass_durations_ms,
                            });
                            self.interval_stats.readback_completed =
                                self.interval_stats.readback_completed.saturating_add(1);
                            let lag_frames = self.latest_frame_id.saturating_sub(frame_id);
                            self.interval_stats.lag_total_frames = self
                                .interval_stats
                                .lag_total_frames
                                .saturating_add(lag_frames);
                            self.interval_stats.lag_max_frames =
                                self.interval_stats.lag_max_frames.max(lag_frames);
                        } else {
                            self.interval_stats.map_failed =
                                self.interval_stats.map_failed.saturating_add(1);
                        }
                    } else {
                        self.interval_stats.map_failed =
                            self.interval_stats.map_failed.saturating_add(1);
                    }
                    drop(range);
                    readback.buffer.unmap();
                    readback.state.store(IDLE, Ordering::SeqCst);
                }
                FAILED => {
                    readback.frame_id = None;
                    readback.pass_mask = 0;
                    readback.state.store(IDLE, Ordering::SeqCst);
                    self.interval_stats.map_failed =
                        self.interval_stats.map_failed.saturating_add(1);
                }
                _ => {}
            }
        }
    }
    pub(crate) fn take_completed_sample(&mut self) -> Option<GpuTimingSample> {
        self.completed_samples.pop_front()
    }
    pub(crate) fn take_interval_stats(&mut self) -> GpuTimerStats {
        std::mem::replace(
            &mut self.interval_stats,
            GpuTimerStats::new(self.sample_interval_frames),
        )
    }
}
fn sample_interval_from_env() -> u32 {
    #[cfg(target_arch = "wasm32")]
    {
        DEFAULT_SAMPLE_INTERVAL_FRAMES
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var(SAMPLE_INTERVAL_ENV)
            .ok()
            .and_then(|value| value.parse::<u32>().ok())
            .map(normalize_sample_interval)
            .unwrap_or(DEFAULT_SAMPLE_INTERVAL_FRAMES)
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn normalize_sample_interval(interval: u32) -> u32 {
    interval.clamp(1, MAX_SAMPLE_INTERVAL_FRAMES)
}
fn frame_is_sampled(frame_id: u64, sample_interval_frames: u32) -> bool {
    frame_id != 0 && frame_id.is_multiple_of(u64::from(sample_interval_frames.max(1)))
}
fn percentage(numerator: u64, denominator: u64) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64 * 100.0
    }
}
fn timestamp_ticks_to_ms(ticks: u64, timestamp_period_ns: f64) -> f64 {
    ticks as f64 * timestamp_period_ns / 1_000_000.0
}
