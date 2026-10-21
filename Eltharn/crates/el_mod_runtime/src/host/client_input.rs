use super::*;
use el_mod_content::scene_types::{InputEvent, InputFrame, SceneCommand, SceneRequest, UiView};

const MAX_EVENT_PAGE: usize = 8;

fn response(value: &impl Serialize, output: &mut [u8]) -> i32 {
    let Ok(bytes) = serde_json::to_vec(value) else {
        return ec(ErrorCode::InvalidRequest);
    };
    if bytes.len() > output.len() {
        return ec(ErrorCode::BufferTooSmall);
    }
    output[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as i32
}

#[derive(Serialize)]
struct Event<'a> {
    sequence: String,
    kind: &'a str,
    target: &'a Option<el_mod_content::scene_types::ObjectRef>,
    widget: Option<u32>,
    key: &'a Option<String>,
    pressed: bool,
    pointer: [i32; 2],
}
impl<'a> From<&'a InputEvent> for Event<'a> {
    fn from(event: &'a InputEvent) -> Self {
        Self {
            sequence: event.sequence.to_string(),
            kind: &event.kind,
            target: &event.target,
            widget: event.widget,
            key: &event.key,
            pressed: event.pressed,
            pointer: event.pointer,
        }
    }
}

impl Host {
    fn client_input_access(&self, capability: &str) -> Result<(), ErrorCode> {
        if !self.has(capability) || self.context.as_ref().is_none_or(|c| c.side != 1) {
            return Err(ErrorCode::CapabilityDenied);
        }
        if !matches!(self.phase_of(), Ok(Phase::WorldEnter | Phase::Tick)) {
            return Err(ErrorCode::WrongPhase);
        }
        Ok(())
    }

    fn routed_input(&self) -> Result<(&InputFrame, String), ErrorCode> {
        self.client_input_access("content.client.read")?;
        let context = self.context.as_ref().unwrap();
        let content = self
            .snapshot
            .content
            .as_ref()
            .ok_or(ErrorCode::Unavailable)?;
        if context.epoch == 0
            || content.scene.epoch != context.epoch
            || content.scene.actor.owner != context.actor
        {
            return Err(ErrorCode::Unavailable);
        }
        Ok((
            &content.input,
            format!(
                "{}|{}|{}",
                context.actor, context.epoch, content.input.sequence
            ),
        ))
    }

    pub(super) fn do_client_input_read(&mut self, request: &[u8], output: &mut [u8]) -> i32 {
        let (input, handle) = match self.routed_input() {
            Ok(value) => value,
            Err(error) => return ec(error),
        };
        if !request.is_empty() {
            return ec(ErrorCode::InvalidRequest);
        }
        response(
            &serde_json::json!({
                "version": 1, "ready": input.sequence != 0, "handle": handle,
                "epoch": self.context.as_ref().unwrap().epoch.to_string(),
                "sequence": input.sequence.to_string(), "viewport": input.viewport,
                "event_count": input.events.len(),
            }),
            output,
        )
    }

    pub(super) fn do_client_input_events(&mut self, request: &[u8], output: &mut [u8]) -> i32 {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Request {
            handle: String,
            offset: usize,
            count: usize,
        }
        let (input, handle) = match self.routed_input() {
            Ok(value) => value,
            Err(error) => return ec(error),
        };
        let request: Request = match strict_json(request, MAX_BUF)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(request) => request,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if request.handle != handle || request.count == 0 || request.count > MAX_EVENT_PAGE {
            return ec(ErrorCode::InvalidRequest);
        }
        let Some(end) = request.offset.checked_add(request.count) else {
            return ec(ErrorCode::InvalidRequest);
        };
        let Some(events) = input.events.get(request.offset..end) else {
            return ec(ErrorCode::InvalidRequest);
        };
        let events: Vec<_> = events.iter().map(Event::from).collect();
        response(
            &serde_json::json!({"version": 1, "handle": handle, "offset": request.offset, "events": events}),
            output,
        )
    }

    pub(super) fn do_client_ui(&mut self, request: &[u8], _output: &mut [u8]) -> i32 {
        if let Err(error) = self.client_input_access("content.client.write") {
            return ec(error);
        }
        let ui: UiView = match strict_json(request, MAX_BUF)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(ui) => ui,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let operation = crate::content::ContentOperation::Apply {
            request: SceneRequest {
                epoch: self.context.as_ref().unwrap().epoch,
                commands: vec![SceneCommand::Ui { view: ui }],
            },
        };
        match self.stage_cmd(Command::Content {
            request: Box::new(operation),
        }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
}
