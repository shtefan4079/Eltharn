#[derive(Debug, Clone)]
pub struct DeltaTrackerTimer {
    delta_ticks: f32,
    delta_tick_residual: f32,
    realtime_delta_ticks: f32,
    paused_delta_tick_residual: f32,
    base_ms_per_tick: f32,
    tick_rate_manager_ms_per_tick: f32,
    paused: bool,
    frozen: bool,
    frozen_ticks_to_run: u32,
    run_game_elements: bool,
}
impl DeltaTrackerTimer {
    pub fn new(ticks_per_second: f32) -> Self {
        assert!(ticks_per_second > 0.0);
        Self {
            delta_ticks: 0.0,
            delta_tick_residual: 0.0,
            realtime_delta_ticks: 0.0,
            paused_delta_tick_residual: 0.0,
            base_ms_per_tick: 1000.0 / ticks_per_second,
            tick_rate_manager_ms_per_tick: 1000.0 / ticks_per_second,
            paused: false,
            frozen: false,
            frozen_ticks_to_run: 0,
            run_game_elements: true,
        }
    }
    pub fn advance_game_time_seconds(&mut self, delta_seconds: f32, has_level: bool) -> u32 {
        let delta_ms = delta_seconds.max(0.0) * 1000.0;
        self.advance_game_time_ms(delta_ms, has_level)
    }
    pub fn advance_game_time_ms(&mut self, delta_ms: f32, has_level: bool) -> u32 {
        self.delta_ticks = delta_ms.max(0.0) / self.game_time_ms_per_tick(has_level);
        self.delta_tick_residual += self.delta_ticks;
        let ticks = self.delta_tick_residual.floor() as u32;
        self.delta_tick_residual -= ticks as f32;
        ticks
    }
    pub fn begin_client_tick(&mut self, has_level: bool) -> bool {
        if has_level {
            self.tick_rate_manager_tick();
        } else {
            self.run_game_elements = true;
        }
        self.run_game_elements
    }
    fn tick_rate_manager_tick(&mut self) {
        self.run_game_elements = !self.frozen || self.frozen_ticks_to_run > 0;
        if self.frozen_ticks_to_run > 0 {
            self.frozen_ticks_to_run -= 1;
        }
    }
    pub fn set_tick_rate(&mut self, tick_rate: f32) {
        let tick_rate = tick_rate.max(1.0);
        let nanoseconds_per_tick = (1_000_000_000.0f64 / tick_rate as f64) as u64;
        self.tick_rate_manager_ms_per_tick = nanoseconds_per_tick as f32 / 1_000_000.0;
    }
    fn game_time_ms_per_tick(&self, has_level: bool) -> f32 {
        if has_level && self.run_game_elements {
            self.base_ms_per_tick
                .max(self.tick_rate_manager_ms_per_tick)
        } else {
            self.base_ms_per_tick
        }
    }
    pub fn advance_real_time_seconds(&mut self, delta_seconds: f32) {
        let delta_ms = delta_seconds.max(0.0) * 1000.0;
        self.advance_real_time_ms(delta_ms);
    }
    pub fn advance_real_time_ms(&mut self, delta_ms: f32) {
        self.realtime_delta_ticks = delta_ms.max(0.0) / self.base_ms_per_tick;
    }
    #[allow(dead_code)]
    pub fn update_pause_state(&mut self, paused: bool) {
        if paused {
            self.pause();
        } else {
            self.unpause();
        }
    }
    #[allow(dead_code)]
    fn pause(&mut self) {
        if !self.paused {
            self.paused_delta_tick_residual = self.delta_tick_residual;
        }
        self.paused = true;
    }
    #[allow(dead_code)]
    fn unpause(&mut self) {
        if self.paused {
            self.delta_tick_residual = self.paused_delta_tick_residual;
        }
        self.paused = false;
    }
    #[allow(dead_code)]
    pub fn update_frozen_state(&mut self, frozen: bool) {
        self.frozen = frozen;
    }
    pub fn set_frozen_ticks_to_run(&mut self, ticks: u32) {
        self.frozen_ticks_to_run = ticks;
    }
    #[allow(dead_code)]
    pub fn frozen_ticks_to_run(&self) -> u32 {
        self.frozen_ticks_to_run
    }
    #[allow(dead_code)]
    pub fn runs_normally(&self) -> bool {
        self.run_game_elements
    }
    #[allow(dead_code)]
    pub fn game_time_delta_ticks(&self) -> f32 {
        self.delta_ticks
    }
    pub fn game_time_delta_partial_tick(&self, ignore_frozen_game: bool) -> f32 {
        if !ignore_frozen_game && self.frozen {
            1.0
        } else if self.paused {
            self.paused_delta_tick_residual
        } else {
            self.delta_tick_residual
        }
    }
    #[allow(dead_code)]
    pub fn realtime_delta_ticks(&self) -> f32 {
        if self.realtime_delta_ticks > 7.0 {
            0.5
        } else {
            self.realtime_delta_ticks
        }
    }
}
