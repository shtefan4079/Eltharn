use super::*;
impl Host {
    pub(super) fn do_app_read(&mut self, request: &[u8], response: &mut [u8]) -> i32 {
        if !request.is_empty() {
            return ec(ErrorCode::InvalidRequest);
        }
        if !self.has(crate::app::READ_CAP) {
            return ec(ErrorCode::CapabilityDenied);
        }
        if !self
            .context
            .as_ref()
            .is_some_and(|c| c.side == 1 && c.phase == Phase::AppTick)
        {
            return ec(ErrorCode::WrongPhase);
        }
        let Some(snapshot) = &self.snapshot.app else {
            return ec(ErrorCode::InvalidRequest);
        };
        let frame = &snapshot.input;
        let bytes = match serde_json::to_vec(
            &serde_json::json!({"sequence":frame.sequence,"context":frame.context,"settings_session":frame.settings_session,"menu":frame.menu,"viewport":frame.viewport,"pointer":frame.pointer,"event_count":frame.events.len(),"widget_count":snapshot.ui.widgets.len()}),
        ) {
            Ok(bytes) => bytes,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if bytes.len() > response.len() {
            return ec(ErrorCode::BufferTooSmall);
        }
        response[..bytes.len()].copy_from_slice(&bytes);
        bytes.len() as i32
    }
    pub(super) fn do_app_event(&mut self, request: &[u8], response: &mut [u8]) -> i32 {
        if !self.has(crate::app::READ_CAP) {
            return ec(ErrorCode::CapabilityDenied);
        }
        if !self
            .context
            .as_ref()
            .is_some_and(|c| c.side == 1 && c.phase == Phase::AppTick)
        {
            return ec(ErrorCode::WrongPhase);
        }
        let Ok([a, b, c, d]) = <[u8; 4]>::try_from(request) else {
            return ec(ErrorCode::InvalidRequest);
        };
        let index = u32::from_le_bytes([a, b, c, d]) as usize;
        let Some(snapshot) = &self.snapshot.app else {
            return ec(ErrorCode::InvalidRequest);
        };
        let Some(event) = snapshot.input.events.get(index) else {
            return ec(ErrorCode::InvalidRequest);
        };
        let bytes = match serde_json::to_vec(event) {
            Ok(bytes) => bytes,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if bytes.len() > response.len() {
            return ec(ErrorCode::BufferTooSmall);
        }
        response[..bytes.len()].copy_from_slice(&bytes);
        bytes.len() as i32
    }
    pub(super) fn do_app_ui(&mut self, request: &[u8], _response: &mut [u8]) -> i32 {
        let ui = match strict_json(request, super::MAX_ITEM)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(ui) => ui,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        match self.stage_cmd(Command::AppUi { ui }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
    pub(super) fn do_app_menu(&mut self, request: &[u8], _response: &mut [u8]) -> i32 {
        let menu: crate::app::Menu = match strict_json(request, super::MAX_ITEM)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(menu) => menu,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let ui = match menu.into_ui() {
            Ok(ui) => ui,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        match self.stage_cmd(Command::AppUi { ui }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
}
