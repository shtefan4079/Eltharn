use super::*;
impl Host {
    pub(super) fn do_canvas(&mut self, request: &[u8], _response: &mut [u8]) -> i32 {
        let command = match strict_json(request, MAX_BUF)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(command) => command,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let Some(context) = self.context.as_ref() else {
            return ec(ErrorCode::WrongPhase);
        };
        let request = crate::content::ContentOperation::Canvas {
            epoch: context.epoch,
            command,
        };
        match self.stage_cmd(Command::Content {
            request: Box::new(request),
        }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
}
