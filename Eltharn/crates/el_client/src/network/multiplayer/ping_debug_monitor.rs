#![allow(dead_code)]
const CAPACITY: usize = 240;
pub struct PingDebugMonitor {
    pub delay_samples: Vec<u64>,
}
impl Default for PingDebugMonitor {
    fn default() -> Self {
        Self::new()
    }
}
impl PingDebugMonitor {
    pub fn new() -> Self {
        Self {
            delay_samples: Vec::new(),
        }
    }
    pub fn log_sample(&mut self, delay_ms: u64) {
        self.delay_samples.push(delay_ms);
        if self.delay_samples.len() > CAPACITY {
            self.delay_samples.remove(0);
        }
    }
    pub fn get_average_delay(&self) -> f64 {
        if self.delay_samples.is_empty() {
            0.0
        } else {
            let sum: u64 = self.delay_samples.iter().sum();
            sum as f64 / self.delay_samples.len() as f64
        }
    }
}
