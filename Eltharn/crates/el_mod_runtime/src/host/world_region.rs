use super::*;
use el_mod_content::scene_types::world_region::{
    MAX_REGION_PAGE, RegionSnapshot, RegionStatus, RegionWatch,
};

fn response(value: &impl Serialize, output: &mut [u8]) -> i32 {
    let bytes = match serde_json::to_vec(value) {
        Ok(bytes) => bytes,
        Err(_) => return ec(ErrorCode::InvalidRequest),
    };
    if bytes.len() > output.len() {
        return ec(ErrorCode::BufferTooSmall);
    }
    output[..bytes.len()].copy_from_slice(&bytes);
    bytes.len() as i32
}

impl Host {
    fn region_access(&self) -> Result<(), ErrorCode> {
        if !self.has("world.blocks.read") || self.context.as_ref().is_none_or(|c| c.side != 1) {
            return Err(ErrorCode::CapabilityDenied);
        }
        if !matches!(self.phase_of(), Ok(Phase::WorldEnter | Phase::Tick)) {
            return Err(ErrorCode::WrongPhase);
        }
        Ok(())
    }
    fn region_snapshot(&self) -> Result<(&RegionWatch, Option<&RegionSnapshot>), ErrorCode> {
        self.region_access()?;
        let content = self
            .snapshot
            .content
            .as_ref()
            .ok_or(ErrorCode::Unavailable)?;
        let watch = content
            .scene
            .actor
            .world_region
            .as_ref()
            .ok_or(ErrorCode::Unavailable)?;
        let context = self.context.as_ref().ok_or(ErrorCode::WrongPhase)?;
        let prefix = format!("{}|{}|{}|", context.actor, context.epoch, watch.query);
        let sample = content
            .input
            .world_region
            .as_ref()
            .filter(|sample| sample.watch == *watch && sample.handle.starts_with(&prefix));
        Ok((watch, sample))
    }
    pub(super) fn do_world_region_watch(&mut self, req: &[u8], _res: &mut [u8]) -> i32 {
        let watch = match strict_json(req, MAX_BUF)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(watch) => watch,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        let Some(context) = self.context.as_ref() else {
            return ec(ErrorCode::WrongPhase);
        };
        let request = crate::content::ContentOperation::WorldRegion {
            epoch: context.epoch,
            watch,
        };
        match self.stage_cmd(Command::Content {
            request: Box::new(request),
        }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
    pub(super) fn do_world_region_read(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        let (watch, sample) = match self.region_snapshot() {
            Ok(value) => value,
            Err(e) => return ec(e),
        };
        if !req.is_empty() {
            return ec(ErrorCode::InvalidRequest);
        }
        let status = sample
            .map(|s| serde_json::to_value(s.status).unwrap())
            .unwrap_or(Value::String("pending".into()));
        response(
            &serde_json::json!({"version":1,"query":watch.query,"origin":watch.origin,
            "size":watch.size,"ready":sample.is_some_and(|s| s.status == RegionStatus::Ready),
            "status":status,"handle":sample.map(|s| &s.handle),"cell_count":watch.cells(),
            "palette_count":sample.map_or(0, |s| s.palette.len())}),
            res,
        )
    }
    pub(super) fn do_world_region_page(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Request {
            handle: String,
            offset: usize,
            count: usize,
        }
        let (_, sample) = match self.region_snapshot() {
            Ok(value) => value,
            Err(e) => return ec(e),
        };
        let Some(sample) = sample.filter(|s| s.status == RegionStatus::Ready) else {
            return ec(ErrorCode::Unavailable);
        };
        let request: Request = match strict_json(req, MAX_BUF)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(request) => request,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if request.handle != sample.handle || request.count == 0 || request.count > MAX_REGION_PAGE
        {
            return ec(ErrorCode::InvalidRequest);
        }
        let Some(end) = request.offset.checked_add(request.count) else {
            return ec(ErrorCode::InvalidRequest);
        };
        let Some(indices) = sample.indices.get(request.offset..end) else {
            return ec(ErrorCode::InvalidRequest);
        };
        response(&indices, res)
    }
    pub(super) fn do_world_region_palette(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Request {
            handle: String,
            index: usize,
        }
        let (_, sample) = match self.region_snapshot() {
            Ok(value) => value,
            Err(e) => return ec(e),
        };
        let Some(sample) = sample.filter(|s| s.status == RegionStatus::Ready) else {
            return ec(ErrorCode::Unavailable);
        };
        let request: Request = match strict_json(req, MAX_BUF)
            .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
        {
            Ok(request) => request,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if request.handle != sample.handle {
            return ec(ErrorCode::InvalidRequest);
        }
        let Some(cell) = sample.palette.get(request.index) else {
            return ec(ErrorCode::InvalidRequest);
        };
        response(cell, res)
    }
}
