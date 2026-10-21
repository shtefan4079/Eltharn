#![allow(dead_code)]

use web_time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    WaitingForServer {
        timeout_after_ms: u64,
    },
    WaitingForPlayerChunk {
        player_section_ready: bool,
        timeout_after_ms: u64,
    },
    ClientLevelReady {
        ready_at_ms: u64,
    },
}

pub struct LevelLoadTracker {
    pub close_delay_ms: u64,
    pub server_progress: f32,
    pub server_stage_active: bool,
    pub client_state: Option<ClientState>,
    pub start_time: Instant,
}

impl Default for LevelLoadTracker {
    fn default() -> Self {
        Self::new(500)
    }
}

impl LevelLoadTracker {
    pub fn new(close_delay_ms: u64) -> Self {
        Self {
            close_delay_ms,
            server_progress: 0.0,
            server_stage_active: false,
            client_state: None,
            start_time: Instant::now(),
        }
    }

    fn current_time_ms(&self) -> u64 {
        self.start_time.elapsed().as_millis() as u64
    }

    pub fn start_client_load(&mut self) {
        let timeout = self.current_time_ms() + 30000;
        self.client_state = Some(ClientState::WaitingForServer {
            timeout_after_ms: timeout,
        });
    }

    pub fn tick_client_load(&mut self) {
        if let Some(state) = self.client_state {
            let now = self.current_time_ms();
            match state {
                ClientState::WaitingForServer { timeout_after_ms } => {
                    if now > timeout_after_ms {
                        self.client_state =
                            Some(ClientState::ClientLevelReady { ready_at_ms: now });
                    }
                }
                ClientState::WaitingForPlayerChunk {
                    player_section_ready,
                    timeout_after_ms,
                } => {
                    if player_section_ready || now > timeout_after_ms {
                        self.client_state =
                            Some(ClientState::ClientLevelReady { ready_at_ms: now });
                    }
                }
                ClientState::ClientLevelReady { .. } => {}
            }
        }
    }

    pub fn is_level_ready(&self) -> bool {
        if let Some(ClientState::ClientLevelReady { ready_at_ms }) = self.client_state {
            self.current_time_ms() >= ready_at_ms + self.close_delay_ms
        } else {
            false
        }
    }

    pub fn loading_packets_received(&mut self) {
        if let Some(ClientState::WaitingForServer { timeout_after_ms }) = self.client_state {
            self.client_state = Some(ClientState::WaitingForPlayerChunk {
                player_section_ready: false,
                timeout_after_ms,
            });
        }
    }

    pub fn set_player_section_ready(&mut self) {
        if let Some(ClientState::WaitingForPlayerChunk {
            timeout_after_ms, ..
        }) = self.client_state
        {
            self.client_state = Some(ClientState::WaitingForPlayerChunk {
                player_section_ready: true,
                timeout_after_ms,
            });
        }
    }
}
