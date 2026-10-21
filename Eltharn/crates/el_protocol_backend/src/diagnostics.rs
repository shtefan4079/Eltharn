use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, mpsc};
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
const DIAGNOSTICS_ENV: &str = "ELTHARN_DIAGNOSTICS";
const DIAGNOSTICS_CONSOLE_ENV: &str = "ELTHARN_DIAGNOSTICS_CONSOLE";
const DIAGNOSTICS_FILE_ENV: &str = "ELTHARN_DIAGNOSTICS_FILE";
const RENDER_PROFILE_ENV: &str = "ELTHARN_RENDER_PROFILE";
const PERFORMANCE_SUMMARY_FILE_ENV: &str = "ELTHARN_PERFORMANCE_SUMMARY_FILE";
const DIAGNOSTICS_MAX_BYTES_ENV: &str = "ELTHARN_DIAGNOSTICS_MAX_BYTES";
const DEFAULT_MAX_LOG_FILE_BYTES: u64 = 25 * 1024 * 1024;
const DEFAULT_LOG_FILE: &str = "eltharn-diagnostics.log";
const DIAGNOSTICS_LOG_QUEUE_CAPACITY: usize = 4_096;
const RENDER_LOG_QUEUE_CAPACITY: usize = 1_024;
const PERFORMANCE_SUMMARY_LOG_QUEUE_CAPACITY: usize = 64;
const LOG_WRITER_BATCH_LIMIT: usize = 256;
const LOG_SHUTDOWN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
static DIAGNOSTICS_LOG_SENDER: OnceLock<AsyncLogSender> = OnceLock::new();
static RENDER_LOG_SENDER: OnceLock<AsyncLogSender> = OnceLock::new();
static PERFORMANCE_SUMMARY_LOG_SENDER: OnceLock<AsyncLogSender> = OnceLock::new();
static LATENCY_TOTALS_LOG_SENDER: OnceLock<AsyncLogSender> = OnceLock::new();
fn env_truthy(name: &str) -> bool {
    std::env::var(name)
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
}
fn env_disabled(name: &str) -> bool {
    std::env::var(name)
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "0" | "false" | "off" | "no"
            )
        })
        .unwrap_or(false)
}
fn requested_game_directory() -> Option<PathBuf> {
    let mut args = std::env::args_os();
    while let Some(arg) = args.next() {
        if arg == "--gameDir" {
            return args.next().map(PathBuf::from);
        }
        let arg = arg.to_string_lossy();
        if let Some(value) = arg.strip_prefix("--gameDir=") {
            return Some(PathBuf::from(value));
        }
    }
    None
}
fn directory_has_entries(path: &PathBuf) -> bool {
    let Ok(entries) = std::fs::read_dir(path) else {
        return false;
    };
    entries
        .flatten()
        .any(|entry| !entry.file_name().to_string_lossy().starts_with('.'))
}
fn game_directory() -> Option<PathBuf> {
    if let Some(path) = requested_game_directory() {
        return Some(path);
    }
    if let Some(dir) =
        std::env::var_os("ELTHARN_GAME_DIR").or_else(|| std::env::var_os("ELTHARN_LOG_DIR"))
    {
        return Some(PathBuf::from(dir));
    }
    let mut candidate_starts = Vec::new();
    if let Ok(current_dir) = std::env::current_dir() {
        candidate_starts.push(current_dir);
    }
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            candidate_starts.push(exe_dir.to_path_buf());
        }
    }
    let fallback = candidate_starts.first().cloned();
    for start in candidate_starts {
        for ancestor in start.ancestors() {
            let resourcepacks = ancestor.join("resourcepacks");
            if directory_has_entries(&resourcepacks) {
                return Some(ancestor.to_path_buf());
            }
            if ancestor.join("options.txt").is_file() {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    fallback
}
pub fn file_enabled() -> bool {
    if cfg!(target_arch = "wasm32") {
        return false;
    }
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| !env_disabled(DIAGNOSTICS_ENV))
}
fn render_profile_log_enabled() -> bool {
    if cfg!(target_arch = "wasm32") {
        return false;
    }
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| !env_disabled(RENDER_PROFILE_ENV))
}
pub fn console_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        env_truthy(DIAGNOSTICS_CONSOLE_ENV)
            || env_truthy(DIAGNOSTICS_ENV)
            || env_truthy("ELTHARN_SLIDE_PROBE_CONSOLE")
            || env_truthy("ELTHARN_NOISY_DEBUG")
            || env_truthy("ELTHARN_VERBOSE_LOGS")
            || matches!(
                std::env::var("ELTHARN_LOGS").as_deref(),
                Ok("all" | "trace" | "debug" | "info")
            )
    })
}
pub fn resource_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        env_truthy("ELTHARN_RESOURCE_DEBUG")
            || env_truthy("ELTHARN_NOISY_DEBUG")
            || env_truthy("ELTHARN_VERBOSE_LOGS")
    })
}
pub fn resource_debug_line(line: &str) {
    if file_enabled() || resource_debug_enabled() {
        enqueue_async_log_block(diagnostics_log_sender(), line);
    }
    if resource_debug_enabled() {
        std::eprintln!("{line}");
    }
}
fn log_path() -> PathBuf {
    if let Some(path) = std::env::var_os(DIAGNOSTICS_FILE_ENV) {
        return PathBuf::from(path);
    }
    game_directory()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(DEFAULT_LOG_FILE)
}
fn utc_timestamp(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let day_seconds = seconds % 86_400;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let month_part = (5 * doy + 2).div_euclid(153);
    let day = doy - (153 * month_part + 2).div_euclid(5) + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    let hour = day_seconds / 3_600;
    let minute = (day_seconds % 3_600) / 60;
    let second = day_seconds % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}Z")
}
pub fn timestamped_line(line: &str) -> String {
    format!("[{}] {}", utc_timestamp(unix_timestamp_seconds()), line)
}
fn unix_timestamp_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
struct AsyncLogSender {
    sender: mpsc::SyncSender<AsyncLogCommand>,
    dropped_blocks: Arc<AtomicU64>,
    next_sequence: AtomicU64,
}
struct AsyncLogBlock {
    emitted_seconds: u64,
    sequence: u64,
    text: String,
}
enum AsyncLogCommand {
    Block(AsyncLogBlock),
    Shutdown(mpsc::Sender<()>),
}
#[derive(Clone, Copy)]
enum AsyncLogFormat {
    TimestampedText,
    JsonLines,
}
#[derive(Clone, Copy)]
struct AsyncLogWriterConfig {
    queue_capacity: usize,
    dropped_field: &'static str,
    backpressure_reason: &'static str,
    format: AsyncLogFormat,
}
struct AsyncLogFileConfig {
    path: PathBuf,
    thread_name: &'static str,
    init_label: &'static str,
    writer: AsyncLogWriterConfig,
}
fn max_log_file_bytes() -> u64 {
    std::env::var(DIAGNOSTICS_MAX_BYTES_ENV)
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(DEFAULT_MAX_LOG_FILE_BYTES)
}
struct ProcessFileLock {
    file: std::fs::File,
}
impl ProcessFileLock {
    fn acquire(lock_path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(lock_path)?;
        file.lock()?;
        Ok(Self { file })
    }
}
impl Drop for ProcessFileLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct FileIdentity {
    #[cfg(unix)]
    dev: u64,
    #[cfg(unix)]
    ino: u64,
    #[cfg(windows)]
    volume_serial_number: u32,
    #[cfg(windows)]
    file_index: u64,
    #[cfg(not(any(unix, windows)))]
    _dummy: (),
}
impl FileIdentity {
    fn from_file(file: &std::fs::File) -> Option<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            file.metadata().ok().map(|m| Self {
                dev: m.dev(),
                ino: m.ino(),
            })
        }
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            #[repr(C)]
            struct FILETIME {
                dw_low_date_time: u32,
                dw_high_date_time: u32,
            }
            #[repr(C)]
            struct BY_HANDLE_FILE_INFORMATION {
                dw_file_attributes: u32,
                ft_creation_time: FILETIME,
                ft_last_access_time: FILETIME,
                ft_last_write_time: FILETIME,
                dw_volume_serial_number: u32,
                n_file_size_high: u32,
                n_file_size_low: u32,
                n_number_of_links: u32,
                n_file_index_high: u32,
                n_file_index_low: u32,
            }
            unsafe extern "system" {
                fn GetFileInformationByHandle(
                    h_file: *mut std::ffi::c_void,
                    lp_file_information: *mut BY_HANDLE_FILE_INFORMATION,
                ) -> i32;
            }
            let mut info = std::mem::MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::uninit();
            let ret = unsafe {
                GetFileInformationByHandle(file.as_raw_handle() as *mut _, info.as_mut_ptr())
            };
            if ret != 0 {
                let info = unsafe { info.assume_init() };
                Some(Self {
                    volume_serial_number: info.dw_volume_serial_number,
                    file_index: ((info.n_file_index_high as u64) << 32)
                        | (info.n_file_index_low as u64),
                })
            } else {
                None
            }
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = file;
            Some(Self { _dummy: () })
        }
    }
    fn from_path(path: &Path) -> Option<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            std::fs::metadata(path).ok().map(|m| Self {
                dev: m.dev(),
                ino: m.ino(),
            })
        }
        #[cfg(windows)]
        {
            let file = OpenOptions::new().read(true).open(path).ok()?;
            Self::from_file(&file)
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = path;
            Some(Self { _dummy: () })
        }
    }
}
pub(crate) struct RotatingFileWriter {
    path: PathBuf,
    lock_path: PathBuf,
    max_size: u64,
    file: Option<std::fs::File>,
    identity: Option<FileIdentity>,
}
impl RotatingFileWriter {
    pub(crate) fn new(path: PathBuf, max_size: u64) -> std::io::Result<Self> {
        if max_size == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "log size limit must be positive; use ELTHARN_DIAGNOSTICS=0 to disable logging",
            ));
        }
        if let Some(parent) = path.parent() {
            let _ = create_dir_all(parent);
        }
        let lock_path = path.with_extension("log.lock");
        let mut slf = Self {
            path,
            lock_path,
            max_size,
            file: None,
            identity: None,
        };
        let _lock = ProcessFileLock::acquire(&slf.lock_path)?;
        let current_size = std::fs::metadata(&slf.path).map(|m| m.len()).unwrap_or(0);
        if current_size >= slf.max_size && current_size > 0 {
            slf.rotate_locked()?;
        } else {
            slf.reopen_locked()?;
        }
        Ok(slf)
    }
    fn reopen_locked(&mut self) -> std::io::Result<()> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        self.identity = FileIdentity::from_file(&file);
        self.file = Some(file);
        Ok(())
    }
    fn rotate_locked(&mut self) -> std::io::Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
        }
        self.identity = None;
        let old_path = self.path.with_extension("log.old");
        std::fs::rename(&self.path, &old_path)?;
        self.reopen_locked()
    }
    #[allow(dead_code)]
    pub(crate) fn write_record(&mut self, record: &[u8]) -> std::io::Result<()> {
        self.write_all(record)
    }
}
impl Write for RotatingFileWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.is_empty() {
            return Ok(0);
        }
        if bytes.len() as u64 > self.max_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "diagnostic record exceeds the configured log size limit",
            ));
        }
        let _lock = ProcessFileLock::acquire(&self.lock_path)?;
        let current_size = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        let path_identity = FileIdentity::from_path(&self.path);
        let should_rotate = current_size > 0
            && (current_size.saturating_add(bytes.len() as u64) > self.max_size
                || current_size >= self.max_size);
        if should_rotate {
            self.rotate_locked()?;
        } else if self.file.is_none() || self.identity != path_identity {
            self.reopen_locked()?;
        }
        if let Some(file) = &mut self.file {
            file.write_all(bytes)?;
            file.flush()?;
            Ok(bytes.len())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "failed to open log file",
            ))
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        let _lock = ProcessFileLock::acquire(&self.lock_path)?;
        if let Some(file) = &mut self.file {
            file.flush()
        } else {
            Ok(())
        }
    }
}
fn spawn_async_file_log(config: AsyncLogFileConfig) -> AsyncLogSender {
    let (sender, receiver) = mpsc::sync_channel::<AsyncLogCommand>(config.writer.queue_capacity);
    let dropped_blocks = Arc::new(AtomicU64::new(0));
    let writer_dropped_blocks = Arc::clone(&dropped_blocks);
    let error_path = config.path.clone();
    if let Err(error) = thread::Builder::new()
        .name(config.thread_name.to_string())
        .spawn(move || {
            let max_bytes = max_log_file_bytes();
            let mut writer = match RotatingFileWriter::new(config.path.clone(), max_bytes) {
                Ok(writer) => writer,
                Err(error) => {
                    std::eprintln!(
                        "[EltharnDiagnostics] failed to open log file {}: {error}",
                        config.path.display()
                    );
                    return;
                }
            };
            if matches!(config.writer.format, AsyncLogFormat::TimestampedText) {
                let _ = write_log_block(
                    &mut writer,
                    &AsyncLogBlock {
                        emitted_seconds: unix_timestamp_seconds(),
                        sequence: 0,
                        text: format!(
                            "[Diagnostics][{}][init] file={} timestamp=UTC-second",
                            config.init_label,
                            config.path.display(),
                        ),
                    },
                    config.writer.format,
                );
                let _ = writer.flush();
            }
            run_async_log_writer(&mut writer, receiver, writer_dropped_blocks, config.writer);
        })
    {
        std::eprintln!(
            "[EltharnDiagnostics] failed to start log writer for {}: {error}",
            error_path.display()
        );
    }
    AsyncLogSender {
        sender,
        dropped_blocks,
        next_sequence: AtomicU64::new(1),
    }
}
fn run_async_log_writer(
    writer: &mut impl Write,
    receiver: mpsc::Receiver<AsyncLogCommand>,
    dropped_blocks: Arc<AtomicU64>,
    config: AsyncLogWriterConfig,
) {
    while let Ok(first) = receiver.recv() {
        let mut shutdown_ack = None;
        let mut last_sequence = None;
        process_async_log_command(
            writer,
            first,
            &mut shutdown_ack,
            &mut last_sequence,
            config.format,
        );
        for _ in 1..LOG_WRITER_BATCH_LIMIT {
            let Ok(command) = receiver.try_recv() else {
                break;
            };
            process_async_log_command(
                writer,
                command,
                &mut shutdown_ack,
                &mut last_sequence,
                config.format,
            );
        }
        let dropped = dropped_blocks.swap(0, Ordering::Relaxed);
        if dropped > 0 {
            let _ = write_log_block(
                writer,
                &AsyncLogBlock {
                    emitted_seconds: unix_timestamp_seconds(),
                    sequence: last_sequence.unwrap_or(0),
                    text: match config.format {
                        AsyncLogFormat::TimestampedText => {
                            format!(
                                "[EltharnDiagnostics] {}={dropped} queue_capacity={} last_written_sequence={} reason={}",
                                config.dropped_field,
                                config.queue_capacity,
                                last_sequence.unwrap_or(0),
                                config.backpressure_reason,
                            )
                        }
                        AsyncLogFormat::JsonLines => {
                            format!(
                                r#"{{"kind":"writer_backpressure","{}":{dropped},"queue_capacity":{},"last_written_sequence":{},"reason":"{}"}}"#,
                                config.dropped_field,
                                config.queue_capacity,
                                last_sequence.unwrap_or(0),
                                config.backpressure_reason,
                            )
                        }
                    },
                },
                config.format,
            );
        }
        let _ = writer.flush();
        if let Some(ack) = shutdown_ack {
            let _ = ack.send(());
            break;
        }
    }
    let _ = writer.flush();
}
fn process_async_log_command(
    writer: &mut impl Write,
    command: AsyncLogCommand,
    shutdown_ack: &mut Option<mpsc::Sender<()>>,
    last_sequence: &mut Option<u64>,
    format: AsyncLogFormat,
) {
    match command {
        AsyncLogCommand::Block(block) => {
            *last_sequence = Some(block.sequence);
            if let Err(error) = write_log_block(writer, &block, format) {
                std::eprintln!("[EltharnDiagnostics] write error: {error}");
            }
        }
        AsyncLogCommand::Shutdown(ack) => *shutdown_ack = Some(ack),
    }
}
fn write_log_block(
    writer: &mut impl Write,
    block: &AsyncLogBlock,
    format: AsyncLogFormat,
) -> std::io::Result<()> {
    match format {
        AsyncLogFormat::TimestampedText => write_timestamped_log_block(writer, block),
        AsyncLogFormat::JsonLines => {
            let mut buffer = Vec::with_capacity(block.text.len() + 1);
            if block.text.ends_with('\n') {
                buffer.extend_from_slice(block.text.as_bytes());
            } else {
                let _ = writeln!(&mut buffer, "{}", block.text);
            }
            writer.write_all(&buffer)
        }
    }
}
fn write_timestamped_log_block(
    writer: &mut impl Write,
    block: &AsyncLogBlock,
) -> std::io::Result<()> {
    let timestamp = utc_timestamp(block.emitted_seconds);
    let mut buffer = Vec::with_capacity(block.text.len() + 32);
    for line in block.text.lines() {
        let _ = writeln!(&mut buffer, "[{timestamp}] {line}");
    }
    if !buffer.is_empty() {
        writer.write_all(&buffer)?;
    }
    Ok(())
}
fn enqueue_async_log_block(log: &AsyncLogSender, text: &str) {
    let sequence = log.next_sequence.fetch_add(1, Ordering::Relaxed);
    let block = AsyncLogBlock {
        emitted_seconds: unix_timestamp_seconds(),
        sequence,
        text: text.to_owned(),
    };
    match log.sender.try_send(AsyncLogCommand::Block(block)) {
        Ok(()) => {}
        Err(mpsc::TrySendError::Full(_)) => {
            log.dropped_blocks.fetch_add(1, Ordering::Relaxed);
        }
        Err(mpsc::TrySendError::Disconnected(_)) => {}
    }
}
fn shutdown_async_log_until(log: &AsyncLogSender, deadline: std::time::Instant) {
    let (ack_tx, ack_rx) = mpsc::channel();
    let mut command = AsyncLogCommand::Shutdown(ack_tx);
    loop {
        match log.sender.try_send(command) {
            Ok(()) => break,
            Err(mpsc::TrySendError::Full(returned)) => {
                if std::time::Instant::now() >= deadline {
                    return;
                }
                command = returned;
                thread::park_timeout(std::time::Duration::from_millis(1));
            }
            Err(mpsc::TrySendError::Disconnected(_)) => return,
        }
    }
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    if !remaining.is_zero() {
        let _ = ack_rx.recv_timeout(remaining);
    }
}
fn diagnostics_log_sender() -> &'static AsyncLogSender {
    DIAGNOSTICS_LOG_SENDER.get_or_init(|| {
        spawn_async_file_log(AsyncLogFileConfig {
            path: log_path(),
            thread_name: "eltharn-diagnostics-log",
            init_label: "Rust",
            writer: AsyncLogWriterConfig {
                queue_capacity: DIAGNOSTICS_LOG_QUEUE_CAPACITY,
                dropped_field: "dropped_lines",
                backpressure_reason: "diagnostics_log_backpressure",
                format: AsyncLogFormat::TimestampedText,
            },
        })
    })
}
pub fn write_line(line: &str) {
    if !file_enabled() {
        return;
    }
    enqueue_async_log_block(diagnostics_log_sender(), line);
}
fn render_log_path() -> PathBuf {
    if let Some(path) = std::env::var_os("ELTHARN_RENDER_LOG_FILE") {
        return PathBuf::from(path);
    }
    game_directory()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("eltharn-render.log")
}
fn performance_summary_log_path() -> PathBuf {
    if let Some(path) = std::env::var_os(PERFORMANCE_SUMMARY_FILE_ENV) {
        return PathBuf::from(path);
    }
    game_directory()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("eltharn-performance-summary.jsonl")
}
fn render_log_sender() -> &'static AsyncLogSender {
    RENDER_LOG_SENDER.get_or_init(|| {
        spawn_async_file_log(AsyncLogFileConfig {
            path: render_log_path(),
            thread_name: "eltharn-render-log",
            init_label: "Render",
            writer: AsyncLogWriterConfig {
                queue_capacity: RENDER_LOG_QUEUE_CAPACITY,
                dropped_field: "dropped_blocks",
                backpressure_reason: "render_log_backpressure",
                format: AsyncLogFormat::TimestampedText,
            },
        })
    })
}
fn performance_summary_log_sender() -> &'static AsyncLogSender {
    PERFORMANCE_SUMMARY_LOG_SENDER.get_or_init(|| {
        spawn_async_file_log(AsyncLogFileConfig {
            path: performance_summary_log_path(),
            thread_name: "eltharn-performance-summary-log",
            init_label: "PerformanceSummary",
            writer: AsyncLogWriterConfig {
                queue_capacity: PERFORMANCE_SUMMARY_LOG_QUEUE_CAPACITY,
                dropped_field: "dropped_summary_blocks",
                backpressure_reason: "performance_summary_log_backpressure",
                format: AsyncLogFormat::JsonLines,
            },
        })
    })
}
fn latency_totals_log_path() -> PathBuf {
    if let Some(path) = std::env::var_os("ELTHARN_LATENCY_TOTALS_FILE") {
        return PathBuf::from(path);
    }
    game_directory()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("eltharn-latency-totals.log")
}
fn latency_totals_log_sender() -> &'static AsyncLogSender {
    LATENCY_TOTALS_LOG_SENDER.get_or_init(|| {
        spawn_async_file_log(AsyncLogFileConfig {
            path: latency_totals_log_path(),
            thread_name: "eltharn-latency-totals-log",
            init_label: "LatencyTotals",
            writer: AsyncLogWriterConfig {
                queue_capacity: 32,
                dropped_field: "dropped_summary_blocks",
                backpressure_reason: "latency_totals_log_backpressure",
                format: AsyncLogFormat::TimestampedText,
            },
        })
    })
}
pub fn write_render_line(line: &str) {
    write_render_block(line);
}
pub fn write_render_block(block: &str) {
    if !file_enabled() && !render_profile_log_enabled() {
        return;
    }
    enqueue_async_log_block(render_log_sender(), block);
}
pub fn write_performance_summary_line(line: &str) {
    if !render_profile_log_enabled() {
        return;
    }
    enqueue_async_log_block(performance_summary_log_sender(), line);
}
pub fn write_latency_totals_block(block: &str) {
    if !render_profile_log_enabled() {
        return;
    }
    enqueue_async_log_block(latency_totals_log_sender(), block);
}
pub fn shutdown_render_log() {
    if cfg!(target_arch = "wasm32") {
        return;
    }
    let deadline = std::time::Instant::now() + LOG_SHUTDOWN_TIMEOUT;
    if let Some(log) = RENDER_LOG_SENDER.get() {
        shutdown_async_log_until(log, deadline);
    }
    if let Some(log) = PERFORMANCE_SUMMARY_LOG_SENDER.get() {
        shutdown_async_log_until(log, deadline);
    }
    if let Some(log) = LATENCY_TOTALS_LOG_SENDER.get() {
        shutdown_async_log_until(log, deadline);
    }
    if let Some(log) = DIAGNOSTICS_LOG_SENDER.get() {
        shutdown_async_log_until(log, deadline);
    }
}
pub fn is_diagnostic_line(line: &str) -> bool {
    let line = line
        .strip_prefix("[EltharnProtocolBackend stderr] ")
        .unwrap_or(line);
    [
        "[MoveDebug]",
        "[MovementDebug]",
        "[SlideDebug]",
        "[SlideProbe]",
        "[NetDebug]",
        "[PhysicsHold]",
        "[ChunkDebug]",
        "[WireDebug]",
        "[ResourcePackDebug]",
        "[ParityProbe]",
        "[ChestDebug]",
    ]
    .iter()
    .any(|prefix| line.starts_with(prefix))
}
pub fn write_diagnostic_line(line: &str) {
    if is_diagnostic_line(line) {
        write_line(line);
    }
}
