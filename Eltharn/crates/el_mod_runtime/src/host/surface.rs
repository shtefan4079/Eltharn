use super::*;
use el_extension_api::services;
use el_mod_content::scene_types::surface::{
    MAX_SURFACE_PAGE, SurfaceColumn, SurfaceSnapshot, SurfaceStatus,
};

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
impl Host {
    pub(super) fn do_entities_read(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        if let Err(error) = self.client_world_read("client.entities.read") {
            return ec(error);
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Page {
            offset: usize,
            count: usize,
            #[serde(default)]
            handle: Option<String>,
        }
        let page: Page = match strict_json(req, 1024)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(page) => page,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if page.count > el_mod_content::scene_types::entities::MAX_PAGE || page.count == 0 {
            return ec(ErrorCode::InvalidRequest);
        }
        let context = self.context.as_ref().unwrap();
        let snapshot = self
            .snapshot
            .content
            .as_ref()
            .and_then(|c| c.input.entities.as_ref())
            .filter(|s| s.epoch == context.epoch.to_string());
        let Some(snapshot) = snapshot else {
            return response(&serde_json::json!({"version":1,"ready":false}), res);
        };
        let handle = format!("{}|{}|{}", context.actor, snapshot.epoch, snapshot.sequence);
        if page.handle.as_ref().is_some_and(|h| h != &handle) {
            return ec(ErrorCode::InvalidRequest);
        }
        let Some(entities) = snapshot.entities.get(
            page.offset
                ..snapshot
                    .entities
                    .len()
                    .min(page.offset.saturating_add(page.count)),
        ) else {
            return ec(ErrorCode::InvalidRequest);
        };
        response(
            &serde_json::json!({"version":1,"ready":true,"handle":handle,"offset":page.offset,
            "count":snapshot.entities.len(),"total_loaded":snapshot.total_loaded,"truncated":snapshot.truncated,"entities":entities}),
            res,
        )
    }
    fn client_world_read(&self, capability: &str) -> Result<(), ErrorCode> {
        if !self.has(capability) || self.context.as_ref().is_none_or(|c| c.side != 1) {
            return Err(ErrorCode::CapabilityDenied);
        }
        if !matches!(self.phase_of(), Ok(Phase::WorldEnter | Phase::Tick)) {
            return Err(ErrorCode::WrongPhase);
        }
        Ok(())
    }
    pub(super) fn do_player_read(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        if let Err(error) = self.client_world_read("client.player.read") {
            return ec(error);
        }
        if !req.is_empty() {
            return ec(ErrorCode::InvalidRequest);
        }
        let epoch = self.context.as_ref().unwrap().epoch.to_string();
        let player = self
            .snapshot
            .content
            .as_ref()
            .and_then(|c| c.input.player.as_ref())
            .filter(|p| p.epoch == epoch);
        let mut value = player
            .map(|p| serde_json::to_value(p).unwrap())
            .unwrap_or(serde_json::json!({"version":1}));
        value["ready"] = Value::Bool(player.is_some());
        response(&value, res)
    }
    fn surface_snapshot(
        &self,
    ) -> Result<
        (
            &el_mod_content::scene_types::surface::SurfaceWatch,
            Option<&SurfaceSnapshot>,
        ),
        ErrorCode,
    > {
        self.client_world_read("world.blocks.read")?;
        let content = self
            .snapshot
            .content
            .as_ref()
            .ok_or(ErrorCode::Unavailable)?;
        let watch = content
            .scene
            .actor
            .world_surface
            .as_ref()
            .ok_or(ErrorCode::Unavailable)?;
        let context = self.context.as_ref().unwrap();
        let prefix = format!("{}|{}|{}|", context.actor, context.epoch, watch.query);
        Ok((
            watch,
            content
                .input
                .world_surface
                .as_ref()
                .filter(|s| s.watch == *watch && s.handle.starts_with(&prefix)),
        ))
    }
    pub(super) fn do_surface(&mut self, opcode: i32, req: &[u8], res: &mut [u8]) -> i32 {
        if let Err(error) = self.client_world_read("world.blocks.read") {
            return ec(error);
        }
        if opcode == services::WORLD_SURFACE_WATCH_OPCODE {
            let watch = match strict_json(req, MAX_BUF)
                .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
            {
                Ok(watch) => watch,
                Err(_) => return ec(ErrorCode::InvalidRequest),
            };
            let request = crate::content::ContentOperation::WorldSurface {
                epoch: self.context.as_ref().unwrap().epoch,
                watch,
            };
            return match self.stage_cmd(Command::Content {
                request: Box::new(request),
            }) {
                Ok(()) => 0,
                Err(e) => ec(e),
            };
        }
        let (watch, sample) = match self.surface_snapshot() {
            Ok(s) => s,
            Err(e) => return ec(e),
        };
        if opcode == services::WORLD_SURFACE_READ_OPCODE {
            if !req.is_empty() {
                return ec(ErrorCode::InvalidRequest);
            }
            return response(
                &serde_json::json!({"version":1,"query":watch.query,"origin":watch.origin,"size":watch.size,
                "status":sample.map(|s|serde_json::to_value(s.status).unwrap()).unwrap_or(Value::String("pending".into())),
                "ready":sample.is_some_and(|s|s.status==SurfaceStatus::Ready),"handle":sample.map(|s|&s.handle),"column_count":watch.columns()}),
                res,
            );
        }
        let Some(sample) = sample.filter(|s| s.status != SurfaceStatus::BudgetLimit) else {
            return ec(ErrorCode::Unavailable);
        };
        if opcode == services::WORLD_SURFACE_PAGE_OPCODE {
            #[derive(Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Page {
                handle: String,
                offset: usize,
                count: usize,
            }
            let request: Page = match strict_json(req, MAX_BUF)
                .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
            {
                Ok(r) => r,
                Err(_) => return ec(ErrorCode::InvalidRequest),
            };
            if request.handle != sample.handle
                || request.count == 0
                || request.count > MAX_SURFACE_PAGE
            {
                return ec(ErrorCode::InvalidRequest);
            }
            let Some(end) = request.offset.checked_add(request.count) else {
                return ec(ErrorCode::InvalidRequest);
            };
            let Some(columns) = sample.columns.get(request.offset..end) else {
                return ec(ErrorCode::InvalidRequest);
            };
            let summaries: Vec<_> = columns
                .iter()
                .map(|c| match c {
                    SurfaceColumn::Pending => serde_json::json!({"status":"pending"}),
                    SurfaceColumn::Unloaded => serde_json::json!({"status":"unloaded"}),
                    SurfaceColumn::Empty => serde_json::json!({"status":"empty"}),
                    SurfaceColumn::Loaded { y, .. } => serde_json::json!({"status":"loaded","y":y}),
                })
                .collect();
            return response(&summaries, res);
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct State {
            handle: String,
            index: usize,
        }
        let request: State = match strict_json(req, MAX_BUF)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(r) => r,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if request.handle != sample.handle {
            return ec(ErrorCode::InvalidRequest);
        }
        match sample.columns.get(request.index) {
            Some(SurfaceColumn::Loaded { state, .. }) => response(state, res),
            Some(_) => ec(ErrorCode::Unavailable),
            None => ec(ErrorCode::InvalidRequest),
        }
    }
}
