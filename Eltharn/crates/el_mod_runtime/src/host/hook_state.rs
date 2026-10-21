use super::{Command, Host, ec};
use crate::{hook_state, wire::strict_json};
use el_extension_api::abi::ErrorCode;

impl Host {
    pub(super) fn do_hook_state(&mut self, request: &[u8]) -> i32 {
        let value = match strict_json(request, hook_state::MAX_BYTES) {
            Ok(value) => value,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        match self.stage_cmd(Command::HookState { value }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
}
