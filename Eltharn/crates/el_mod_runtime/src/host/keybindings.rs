use super::*;
impl Host {
    pub(super) fn do_keybindings_register(&mut self, request: &[u8], _: &mut [u8]) -> i32 {
        let bindings = match strict_json(request, super::MAX_ITEM)
            .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()))
        {
            Ok(bindings) => bindings,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        match self.stage_cmd(Command::KeybindingsRegister { bindings }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
    pub(super) fn do_keybinding_read(&mut self, request: &[u8], response: &mut [u8]) -> i32 {
        if !self.has(crate::keybindings::CAPABILITY) {
            return ec(ErrorCode::CapabilityDenied);
        }
        if !self
            .context
            .as_ref()
            .is_some_and(|c| c.side == 1 && matches!(c.phase, Phase::Tick | Phase::AppTick))
        {
            return ec(ErrorCode::WrongPhase);
        }
        let Ok(id) = std::str::from_utf8(request) else {
            return ec(ErrorCode::InvalidRequest);
        };
        if crate::keybindings::validate_id(id).is_err() {
            return ec(ErrorCode::InvalidRequest);
        }
        let Some(state) = self
            .snapshot
            .keybindings
            .as_ref()
            .and_then(|s| s.states.iter().find(|s| s.id == id))
        else {
            return ec(ErrorCode::InvalidRequest);
        };
        let Ok(bytes) = serde_json::to_vec(state) else {
            return ec(ErrorCode::InvalidRequest);
        };
        if bytes.len() > response.len() {
            return ec(ErrorCode::BufferTooSmall);
        }
        response[..bytes.len()].copy_from_slice(&bytes);
        bytes.len() as i32
    }
}
