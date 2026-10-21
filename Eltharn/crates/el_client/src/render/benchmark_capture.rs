use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;
use web_time::Instant;
const BENCHMARK_SCHEMA: &str = "eltharn.benchmark.v1";
const OUTPUT_ENV: &str = "ELTHARN_BENCHMARK_OUTPUT";
const START_FILE_ENV: &str = "ELTHARN_BENCHMARK_START_FILE";
const DURATION_ENV: &str = "ELTHARN_BENCHMARK_DURATION_MS";
const MAX_FRAMES_ENV: &str = "ELTHARN_BENCHMARK_MAX_FRAMES";
const DEFAULT_DURATION_MS: u64 = 360_000;
const MAX_DURATION_MS: u64 = 360_000;
const DEFAULT_MAX_FRAMES: usize = 200_000;
const MAX_MAX_FRAMES: u64 = 1_000_000;
const MARKER_POLL_INTERVAL: Duration = Duration::from_millis(500);
const CADENCE_DISCONTINUITY_MS: f64 = 1_000.0;
fn ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1_000.0
}
fn sample_values_valid(work: FrameWork, gpu: Option<(u64, f64)>) -> bool {
    work.work_ms.is_finite()
        && work.work_ms >= 0.0
        && work.cpu_work_ms.is_finite()
        && work.cpu_work_ms >= 0.0
        && work.present_wait_ms.is_finite()
        && work.present_wait_ms >= 0.0
        && work.residual_ms.is_finite()
        && work.residual_ms >= 0.0
        && gpu.is_none_or(|(_, duration_ms)| duration_ms.is_finite() && duration_ms >= 0.0)
}
fn bench_diag(message: &str) {
    eprintln!("{message}");
}
#[derive(Clone, Debug)]
pub(crate) struct BenchmarkConfig {
    output_path: PathBuf,
    start_path: PathBuf,
    requested: Duration,
    requested_ms: u64,
    max_frames: usize,
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct BenchmarkConfigInput<'a> {
    pub(crate) output: Option<&'a str>,
    pub(crate) start_file: Option<&'a str>,
    pub(crate) duration_ms: Option<&'a str>,
    pub(crate) max_frames: Option<&'a str>,
}
pub(crate) fn parse_benchmark_config(
    input: BenchmarkConfigInput<'_>,
) -> Result<Option<BenchmarkConfig>, String> {
    let Some(output) = input.output else {
        return Ok(None);
    };
    if output.trim().is_empty() {
        return Err(format!("{OUTPUT_ENV} is empty"));
    }
    let Some(start_file) = input.start_file else {
        return Err(format!("{OUTPUT_ENV} is set without {START_FILE_ENV}"));
    };
    if start_file.trim().is_empty() {
        return Err(format!("{START_FILE_ENV} is empty"));
    }
    let requested_ms = match input.duration_ms {
        None => DEFAULT_DURATION_MS,
        Some(raw) => parse_bounded_u64(DURATION_ENV, raw, 1, MAX_DURATION_MS)?,
    };
    let max_frames = match input.max_frames {
        None => DEFAULT_MAX_FRAMES,
        Some(raw) => parse_bounded_u64(MAX_FRAMES_ENV, raw, 1, MAX_MAX_FRAMES)? as usize,
    };
    Ok(Some(BenchmarkConfig {
        output_path: PathBuf::from(output),
        start_path: PathBuf::from(start_file),
        requested: Duration::from_millis(requested_ms),
        requested_ms,
        max_frames,
    }))
}
fn parse_bounded_u64(name: &str, raw: &str, min: u64, max: u64) -> Result<u64, String> {
    let value: u64 = raw
        .trim()
        .parse()
        .map_err(|_| format!("{name}={raw:?} is not a positive integer"))?;
    if value < min || value > max {
        return Err(format!("{name}={value} out of range {min}..={max}"));
    }
    Ok(value)
}
pub(crate) fn read_config_from_env() -> Result<Option<BenchmarkConfig>, String> {
    let output = std::env::var(OUTPUT_ENV).ok();
    let start_file = std::env::var(START_FILE_ENV).ok();
    let duration_ms = std::env::var(DURATION_ENV).ok();
    let max_frames = std::env::var(MAX_FRAMES_ENV).ok();
    parse_benchmark_config(BenchmarkConfigInput {
        output: output.as_deref(),
        start_file: start_file.as_deref(),
        duration_ms: duration_ms.as_deref(),
        max_frames: max_frames.as_deref(),
    })
}
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FrameWork {
    pub(crate) work_ms: f64,
    pub(crate) cpu_work_ms: f64,
    pub(crate) present_wait_ms: f64,
    pub(crate) residual_ms: f64,
}
#[derive(Clone, Copy, Debug)]
struct CapturedFrame {
    frame_id: u64,
    start: Duration,
    end: Option<Duration>,
    work: Option<FrameWork>,
    cadence: Option<Duration>,
    gpu_duration_ms: Option<f64>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Arming,
    Capturing,
    Closed,
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum BeginOutcome {
    Noop,
    Waiting,
    Started,
    Recorded,
    Deferred,
    Refused(&'static str),
    Export(serde_json::Value),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EndOutcome {
    Noop,
    Recorded,
    Deferred,
}
pub(crate) struct BenchmarkCore {
    requested: Duration,
    requested_ms: u64,
    max_frames: usize,
    stage: Stage,
    first_frame_id: u64,
    start_elapsed: Duration,
    last_poll_elapsed: Duration,
    last_begin_elapsed: Duration,
    frames: Vec<CapturedFrame>,
    gpu_out_of_window_dropped: u64,
    gpu_duplicate_dropped: u64,
    discontinuities: u64,
    dropped_frames: u64,
    status_complete: bool,
    reason: Option<&'static str>,
    actual_elapsed: Duration,
    export_due: bool,
}
impl BenchmarkCore {
    pub(crate) fn armed(
        requested: Duration,
        requested_ms: u64,
        max_frames: usize,
        marker_preexisting: bool,
        output_preexisting: bool,
    ) -> Result<Self, &'static str> {
        if marker_preexisting {
            return Err("stale_start_marker");
        }
        if output_preexisting {
            return Err("output_path_exists");
        }
        Ok(Self {
            requested,
            requested_ms,
            max_frames,
            stage: Stage::Arming,
            first_frame_id: 0,
            start_elapsed: Duration::ZERO,
            last_poll_elapsed: Duration::ZERO,
            last_begin_elapsed: Duration::ZERO,
            frames: Vec::new(),
            gpu_out_of_window_dropped: 0,
            gpu_duplicate_dropped: 0,
            discontinuities: 0,
            dropped_frames: 0,
            status_complete: false,
            reason: None,
            actual_elapsed: Duration::ZERO,
            export_due: false,
        })
    }
    pub(crate) fn stage_is_arming(&self) -> bool {
        self.stage == Stage::Arming
    }
    pub(crate) fn stage_is_capturing(&self) -> bool {
        self.stage == Stage::Capturing
    }
    pub(crate) fn poll_due(&self, elapsed: Duration) -> bool {
        self.stage == Stage::Arming
            && elapsed.saturating_sub(self.last_poll_elapsed) >= MARKER_POLL_INTERVAL
    }
    fn close_complete(&mut self, elapsed: Duration) {
        self.stage = Stage::Closed;
        self.status_complete = true;
        self.reason = None;
        self.actual_elapsed = elapsed.saturating_sub(self.start_elapsed);
        self.export_due = false;
    }
    fn close_incomplete(&mut self, reason: &'static str, elapsed: Duration) {
        self.stage = Stage::Closed;
        self.status_complete = false;
        self.reason = Some(reason);
        self.actual_elapsed = elapsed.saturating_sub(self.start_elapsed);
        self.export_due = false;
    }
    pub(crate) fn on_begin(
        &mut self,
        frame_id: u64,
        elapsed: Duration,
        marker_present: bool,
        output_exists: bool,
    ) -> BeginOutcome {
        if self.stage == Stage::Closed {
            if self.export_due {
                self.export_due = false;
                return BeginOutcome::Export(self.export_value());
            }
            return BeginOutcome::Noop;
        }
        if self.stage == Stage::Arming {
            if elapsed.saturating_sub(self.last_poll_elapsed) < MARKER_POLL_INTERVAL {
                return BeginOutcome::Waiting;
            }
            self.last_poll_elapsed = elapsed;
            if !marker_present {
                return BeginOutcome::Waiting;
            }
            if output_exists {
                self.close_incomplete("output_path_exists", elapsed);
                return BeginOutcome::Refused("output_path_exists");
            }
            self.stage = Stage::Capturing;
            self.first_frame_id = frame_id;
            self.start_elapsed = elapsed;
            self.last_begin_elapsed = elapsed;
            self.frames = Vec::with_capacity(self.max_frames);
            self.frames.push(CapturedFrame {
                frame_id,
                start: Duration::ZERO,
                end: None,
                work: None,
                cadence: None,
                gpu_duration_ms: None,
            });
            return BeginOutcome::Started;
        }
        if elapsed < self.last_begin_elapsed {
            self.close_incomplete("non_monotonic_clock", elapsed);
            return BeginOutcome::Export(self.export_value());
        }
        let cadence = elapsed.saturating_sub(self.last_begin_elapsed);
        self.last_begin_elapsed = elapsed;
        if let Some(previous) = self.frames.last_mut() {
            if previous.frame_id.wrapping_add(1) != frame_id {
                self.close_incomplete("frame_id_discontinuity", elapsed);
                return BeginOutcome::Export(self.export_value());
            }
            previous.cadence = Some(cadence);
            if ms(cadence) >= CADENCE_DISCONTINUITY_MS {
                self.discontinuities = self.discontinuities.saturating_add(1);
            }
        }
        let since_start = elapsed.saturating_sub(self.start_elapsed);
        if since_start >= self.requested {
            if self.frames.iter().any(|frame| frame.end.is_none()) {
                self.close_incomplete("missing_end_frame", elapsed);
            } else if self.dropped_frames > 0 {
                self.close_incomplete("dropped_frame_records", elapsed);
            } else if self.discontinuities > 0 {
                self.close_incomplete("cadence_discontinuity_in_window", elapsed);
            } else {
                self.close_complete(elapsed);
            }
            return BeginOutcome::Export(self.export_value());
        }
        if self.frames.len() >= self.max_frames {
            self.dropped_frames = self.dropped_frames.saturating_add(1);
            self.close_incomplete("overflow_frame_buffer", elapsed);
            self.export_due = true;
            return BeginOutcome::Deferred;
        }
        self.frames.push(CapturedFrame {
            frame_id,
            start: since_start,
            end: None,
            work: None,
            cadence: None,
            gpu_duration_ms: None,
        });
        BeginOutcome::Recorded
    }
    pub(crate) fn on_end(
        &mut self,
        frame_id: u64,
        work: FrameWork,
        gpu: Option<(u64, f64)>,
        elapsed: Duration,
    ) -> EndOutcome {
        if self.stage != Stage::Capturing {
            return EndOutcome::Noop;
        }
        if elapsed < self.last_begin_elapsed {
            self.close_incomplete("non_monotonic_clock", elapsed);
            self.export_due = true;
            return EndOutcome::Deferred;
        }
        let offset = elapsed.saturating_sub(self.start_elapsed);
        if !sample_values_valid(work, gpu) {
            self.close_incomplete("invalid_sample_values", elapsed);
            self.export_due = true;
            return EndOutcome::Deferred;
        }
        match self.frames.last_mut() {
            Some(frame) if frame.frame_id == frame_id => {
                if frame.end.is_some() {
                    self.close_incomplete("duplicate_end_frame", elapsed);
                    self.export_due = true;
                    return EndOutcome::Deferred;
                }
                frame.end = Some(offset);
                frame.work = Some(work);
            }
            _ => {
                self.dropped_frames = self.dropped_frames.saturating_add(1);
            }
        }
        if let Some((gpu_frame_id, duration_ms)) = gpu {
            let in_window = gpu_frame_id >= self.first_frame_id
                && gpu_frame_id < self.first_frame_id.wrapping_add(self.frames.len() as u64);
            if !in_window {
                self.gpu_out_of_window_dropped = self.gpu_out_of_window_dropped.saturating_add(1);
            } else {
                let slot = (gpu_frame_id - self.first_frame_id) as usize;
                let slot_matches = self
                    .frames
                    .get(slot)
                    .is_some_and(|frame| frame.frame_id == gpu_frame_id);
                if !slot_matches {
                    self.gpu_out_of_window_dropped =
                        self.gpu_out_of_window_dropped.saturating_add(1);
                } else if self.frames[slot].gpu_duration_ms.is_some() {
                    self.gpu_duplicate_dropped = self.gpu_duplicate_dropped.saturating_add(1);
                } else {
                    self.frames[slot].gpu_duration_ms = Some(duration_ms);
                }
            }
        }
        EndOutcome::Recorded
    }
    pub(crate) fn on_shutdown(&mut self, elapsed: Duration) -> Option<serde_json::Value> {
        match self.stage {
            Stage::Arming => {
                self.stage = Stage::Closed;
                self.reason = Some("never_started");
                None
            }
            Stage::Capturing => {
                self.close_incomplete("shutdown_before_window_end", elapsed);
                Some(self.export_value())
            }
            Stage::Closed => {
                if self.export_due {
                    self.export_due = false;
                    Some(self.export_value())
                } else {
                    None
                }
            }
        }
    }
    pub(crate) fn export_value(&self) -> serde_json::Value {
        let frames = self
            .frames
            .iter()
            .map(|frame| {
                serde_json::json!(
                    { "frame_id" : frame.frame_id, "start_ms" : ms(frame.start), "end_ms"
                    : frame.end.map(ms), "work_ms" : frame.work.map(| work | work
                    .work_ms), "cpu_work_ms" : frame.work.map(| work | work.cpu_work_ms),
                    "present_wait_ms" : frame.work.map(| work | work.present_wait_ms),
                    "residual_ms" : frame.work.map(| work | work.residual_ms),
                    "cadence_ms" : frame.cadence.map(ms), }
                )
            })
            .collect::<Vec<_>>();
        let gpu_samples = self
            .frames
            .iter()
            .filter_map(|frame| {
                frame.gpu_duration_ms.map(|duration_ms| {
                    serde_json::json!(
                        { "frame_id" : frame.frame_id, "duration_ms" : duration_ms, }
                    )
                })
            })
            .collect::<Vec<_>>();
        serde_json::json!(
            { "schema" : BENCHMARK_SCHEMA, "status" : if self.status_complete {
            "complete" } else { "incomplete" }, "reason" : self.reason,
            "requested_duration_ms" : self.requested_ms, "actual_duration_ms" : ms(self
            .actual_elapsed), "dropped_frames" : self.dropped_frames, "discontinuities" :
            self.discontinuities, "frames" : frames, "gpu_samples" : gpu_samples,
            "metadata" : { "first_frame_id" : self.first_frame_id, "last_frame_id" : self
            .frames.last().map(| frame | frame.frame_id), "frame_records" : self.frames
            .len(), "max_frames" : self.max_frames, "gpu_out_of_window_dropped" : self
            .gpu_out_of_window_dropped, "gpu_duplicate_dropped" : self
            .gpu_duplicate_dropped, "cadence_discontinuity_threshold_ms" :
            CADENCE_DISCONTINUITY_MS, "cadence_semantics" :
            "begin-to-begin delta of the same monotonic elapsed clock that defines start_ms; the profiler f32-dt input cadence is intentionally unused here and stays in cumulative statistics only; start[i]+cadence[i]==start[i+1]",
            "prefix_observer_cost" :
            "frame buffer (max_frames slots) reserved and one stderr START line emitted at capture start after start_elapsed is fixed and before the first measured frame start_ms=0; the first raw cadence therefore includes this one-time prefix cost: observable, dropped by runner warmup, not auto-excluded; zero per-frame I/O while recording",
            "cpu_semantics" :
            "wall work of existing State::update + State::render minus known surface/present waits; not process utilization",
            "gpu_semantics" :
            "encoded sampled timestamp span keyed by originating frame_id; missing samples are absent, never zero-filled",
            }, }
        )
    }
}
pub(crate) struct BenchmarkCollector {
    core: BenchmarkCore,
    output_path: PathBuf,
    start_path: PathBuf,
    boot: Instant,
}
impl BenchmarkCollector {
    pub(crate) fn from_env_enabled(profiler_enabled: bool) -> Option<Self> {
        if !profiler_enabled {
            return None;
        }
        let config = match read_config_from_env() {
            Ok(None) => return None,
            Err(reason) => {
                bench_diag(&format!(
                    "[EltharnBenchmark][CONFIG_ERROR] {reason}; capture disabled, no file written"
                ));
                return None;
            }
            Ok(Some(config)) => config,
        };
        let marker_preexisting = config.start_path.exists();
        let output_preexisting = config.output_path.exists();
        match BenchmarkCore::armed(
            config.requested,
            config.requested_ms,
            config.max_frames,
            marker_preexisting,
            output_preexisting,
        ) {
            Err(reason) => {
                bench_diag(&format!(
                    "[EltharnBenchmark][REFUSED] reason={reason}; capture disabled, no file written"
                ));
                None
            }
            Ok(core) => Some(Self {
                core,
                output_path: config.output_path,
                start_path: config.start_path,
                boot: Instant::now(),
            }),
        }
    }
    pub(crate) fn on_begin_frame(&mut self, frame_id: u64, now: Instant) {
        let elapsed = now.duration_since(self.boot);
        let (marker_present, output_exists) =
            if self.core.stage_is_arming() && self.core.poll_due(elapsed) {
                (self.start_path.exists(), self.output_path.exists())
            } else {
                (false, false)
            };
        match self
            .core
            .on_begin(frame_id, elapsed, marker_present, output_exists)
        {
            BeginOutcome::Started => bench_diag(&format!(
                "[EltharnBenchmark][START] frame={frame_id} output={:?}",
                self.output_path
            )),
            BeginOutcome::Refused(reason) => bench_diag(&format!(
                "[EltharnBenchmark][REFUSED] reason={reason}; capture disabled, no file written"
            )),
            BeginOutcome::Export(value) => self.export_once(&value),
            BeginOutcome::Noop
            | BeginOutcome::Waiting
            | BeginOutcome::Recorded
            | BeginOutcome::Deferred => {}
        }
    }
    pub(crate) fn on_end_frame(
        &mut self,
        frame_id: u64,
        work: FrameWork,
        gpu: Option<(u64, f64)>,
        now: Instant,
    ) {
        if !self.core.stage_is_capturing() {
            return;
        }
        let elapsed = now.duration_since(self.boot);
        let _ = self.core.on_end(frame_id, work, gpu, elapsed);
    }
    pub(crate) fn on_finish_session(&mut self) {
        let elapsed = self.boot.elapsed();
        if let Some(value) = self.core.on_shutdown(elapsed) {
            self.export_once(&value);
        }
    }
    fn export_once(&self, value: &serde_json::Value) {
        match write_json_once(value, &self.output_path) {
            Ok(()) => bench_diag(&format!(
                "[EltharnBenchmark][EXPORT] status={} path={:?}",
                value["status"], self.output_path
            )),
            Err(error) => bench_diag(&format!(
                "[EltharnBenchmark][EXPORT_ERROR] path={:?} error={error}; file not overwritten",
                self.output_path
            )),
        }
    }
}
pub(crate) fn write_json_once(value: &serde_json::Value, path: &Path) -> std::io::Result<()> {
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::to_writer(&mut writer, value)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))?;
    writer.flush()?;
    Ok(())
}
