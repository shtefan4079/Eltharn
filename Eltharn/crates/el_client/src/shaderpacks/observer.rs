use serde::Serialize;
use serde_json::{Value, json};
use std::cell::RefCell;
const MAX_SAMPLES: usize = 256;
const WARMUP_WORLD_FRAMES: u64 = 20;
#[derive(Serialize, Clone, Copy, Default)]
struct Sample {
    frame_us: u64,
    shader_us: u64,
}
struct Observer {
    active: bool,
    mode: String,
    status: String,
    gpu: Value,
    error: Option<String>,
    presents: u64,
    world_presents: u64,
    shader_frames: u64,
    samples: Vec<Sample>,
}
impl Default for Observer {
    fn default() -> Self {
        Self {
            active: false,
            mode: String::from("off"),
            status: String::from("unconfigured"),
            gpu: Value::Null,
            error: None,
            presents: 0,
            world_presents: 0,
            shader_frames: 0,
            samples: Vec::new(),
        }
    }
}
thread_local! {
    static OBSERVER : RefCell < Observer > = RefCell::new(Observer::default());
}
pub(super) fn configure() {
    OBSERVER.with(|c| {
        let mut o = c.borrow_mut();
        o.active = true;
        if o.samples.capacity() < MAX_SAMPLES {
            let additional = MAX_SAMPLES - o.samples.len();
            o.samples.reserve(additional);
        }
    });
}
pub(super) fn publish(mode: &str, status: &str, gpu: Value, error: Option<String>) {
    OBSERVER.with(|c| {
        let mut o = c.borrow_mut();
        if o.mode != mode {
            o.world_presents = 0;
            o.shader_frames = 0;
            o.samples.clear();
        }
        o.mode = mode.to_owned();
        o.status = status.to_owned();
        o.gpu = gpu;
        o.error = error;
    });
}
pub(super) fn present(in_world: bool, frame_us: u64, shader_us: u64, shader_frames: u64) {
    OBSERVER.with(|c| {
        let mut o = c.borrow_mut();
        if !o.active {
            return;
        }
        o.presents = o.presents.wrapping_add(1);
        o.shader_frames = shader_frames;
        if !in_world {
            return;
        }
        o.world_presents = o.world_presents.wrapping_add(1);
        if o.world_presents <= WARMUP_WORLD_FRAMES {
            return;
        }
        if o.samples.len() >= MAX_SAMPLES {
            return;
        }
        if o.samples.len() >= o.samples.capacity() {
            return;
        }
        o.samples.push(Sample {
            frame_us,
            shader_us,
        });
    });
}
pub(super) fn active() -> bool {
    OBSERVER.with(|c| c.borrow().active)
}
pub(super) fn report() -> Value {
    OBSERVER.with(|c| {
        let o = c.borrow();
        json!(
            { "configured" : o.active, "mode" : o.mode, "status" : o.status, "error"
            : o.error, "gpu" : o.gpu, "presents" : o.presents, "world_presents" : o
            .world_presents, "shader_frames" : o.shader_frames, "samples" : & o
            .samples, "timing_kind" :
            "CPU frame including pacing; shader CPU encode; not GPU time",
            "world_sample_gate" :
            "level exists and active screen is absent; excludes loading and menus",
            "gpu_snapshot_kind" :
            "resources at last prepare/resize; live frame count is shader_frames" }
        )
    })
}
