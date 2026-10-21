#![allow(dead_code)]

pub struct ChunkBatchSizeCalculator {
    aggregated_nanos_per_chunk: f64,
    old_samples_weight: i32,
    chunk_batch_start_time: u64,
}

impl Default for ChunkBatchSizeCalculator {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkBatchSizeCalculator {
    pub fn new() -> Self {
        Self {
            aggregated_nanos_per_chunk: 2000000.0,
            old_samples_weight: 1,
            chunk_batch_start_time: web_time::Instant::now().elapsed().as_nanos() as u64,
        }
    }

    pub fn on_batch_start(&mut self) {
        self.chunk_batch_start_time = web_time::Instant::now().elapsed().as_nanos() as u64;
    }

    pub fn on_batch_finished(&mut self, batch_size: i32) {
        if batch_size > 0 {
            let now = web_time::Instant::now().elapsed().as_nanos() as u64;
            let batch_duration = now.saturating_sub(self.chunk_batch_start_time) as f64;
            let nanos_per_chunk = batch_duration / batch_size as f64;
            let clamped_nanos_per_chunk = nanos_per_chunk.clamp(
                self.aggregated_nanos_per_chunk / 3.0,
                self.aggregated_nanos_per_chunk * 3.0,
            );
            self.aggregated_nanos_per_chunk = (self.aggregated_nanos_per_chunk
                * self.old_samples_weight as f64
                + clamped_nanos_per_chunk)
                / (self.old_samples_weight + 1) as f64;
            self.old_samples_weight = (self.old_samples_weight + 1).min(49);
        }
    }

    pub fn get_desired_chunks_per_tick(&self) -> f32 {
        (7000000.0 / self.aggregated_nanos_per_chunk) as f32
    }
}
