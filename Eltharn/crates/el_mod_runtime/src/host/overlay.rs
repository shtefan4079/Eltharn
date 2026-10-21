use super::{Command, Host, ec};
use crate::{overlay, wire::strict_json};
use el_extension_api::abi::ErrorCode;

impl Host {
    pub(super) fn do_overlay_write(&mut self, request: &[u8], _: &mut [u8]) -> i32 {
        let segments = match strict_json(request, overlay::MAX_REQUEST_BYTES)
            .and_then(|value| serde_json::from_value(value).map_err(|e| e.to_string()))
        {
            Ok(segments) => segments,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        match self.stage_cmd(Command::OverlayMessage { segments }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
}
