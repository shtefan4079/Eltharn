use std::sync::{Arc, mpsc};
pub struct V4ResizeCompletion {
    pub generation: u64,
    pub result: Result<(), String>,
    pub runtime: el_shaderpack::run_v4::RuntimeV4,
}
#[cfg(not(target_arch = "wasm32"))]
const _: fn() = || {
    fn assert_send<T: Send>() {}
    assert_send::<V4ResizeCompletion>();
};
#[derive(Debug, Default)]
pub struct V4ResizeTracker {
    generation: u64,
    in_flight: Option<u64>,
    pending: Option<([u32; 2], u64)>,
    disable_epoch: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub enum V4ResizeRequest {
    Launch { size: [u32; 2], generation: u64 },
    Coalesced,
}
#[derive(Debug, PartialEq, Eq)]
pub enum V4Completion {
    Cancelled,
    FreshSuccess,
    FreshFailure,
    Chain { size: [u32; 2], generation: u64 },
}
impl V4ResizeTracker {
    pub fn request(
        &mut self,
        size: [u32; 2],
        active: bool,
        faulted: bool,
    ) -> Option<V4ResizeRequest> {
        if faulted || !active {
            return None;
        }
        self.generation = self.generation.wrapping_add(1);
        let generation = self.generation;
        if self.in_flight.is_some() {
            self.pending = Some((size, generation));
            Some(V4ResizeRequest::Coalesced)
        } else {
            self.in_flight = Some(generation);
            Some(V4ResizeRequest::Launch { size, generation })
        }
    }
    pub fn in_flight(&self) -> Option<u64> {
        self.in_flight
    }
    pub fn note_disable(&mut self) {
        self.disable_epoch = self.generation;
        self.pending = None;
    }
    pub fn complete(&mut self, generation: u64, failed: bool) -> V4Completion {
        if generation <= self.disable_epoch {
            self.in_flight = None;
            self.pending = None;
            return V4Completion::Cancelled;
        }
        if generation != self.generation {
            debug_assert!(
                self.pending.is_some(),
                "v4 resize: stale completion without coalesced pending size"
            );
            return match self.pending.take() {
                Some((size, pending_generation)) => {
                    self.in_flight = Some(pending_generation);
                    V4Completion::Chain {
                        size,
                        generation: pending_generation,
                    }
                }
                None => {
                    self.in_flight = None;
                    V4Completion::Cancelled
                }
            };
        }
        self.in_flight = None;
        debug_assert!(
            self.pending.is_none(),
            "v4 resize: fresh completion with coalesced pending size"
        );
        if failed {
            V4Completion::FreshFailure
        } else {
            V4Completion::FreshSuccess
        }
    }
}
pub fn spawn_v4_resize(
    runtime: el_shaderpack::run_v4::RuntimeV4,
    device: Arc<wgpu::Device>,
    adapter: Arc<wgpu::Adapter>,
    size: [u32; 2],
    budget: u64,
    generation: u64,
    sender: mpsc::Sender<V4ResizeCompletion>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        wasm_bindgen_futures::spawn_local(async move {
            let mut runtime = runtime;
            let result = runtime.resize(&device, &adapter, size, budget).await;
            let _ = sender.send(V4ResizeCompletion {
                generation,
                result,
                runtime,
            });
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::thread::spawn(move || {
            let mut runtime = runtime;
            let result = pollster::block_on(runtime.resize(&device, &adapter, size, budget));
            let _ = sender.send(V4ResizeCompletion {
                generation,
                result,
                runtime,
            });
        });
    }
}
