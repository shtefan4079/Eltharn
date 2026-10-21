use super::*;
use el_mod_content::scene_types::avatar::{CapeDeformation, LocalAvatarContext};

impl Host {
    fn avatar_service(&self, capability: &str) -> Result<(), ErrorCode> {
        if !self.has(capability) || self.context.as_ref().is_none_or(|c| c.side != 1) {
            return Err(ErrorCode::CapabilityDenied);
        }
        if !matches!(self.phase_of(), Ok(Phase::WorldEnter | Phase::Tick)) {
            return Err(ErrorCode::WrongPhase);
        }
        Ok(())
    }
    fn local_avatar(&self) -> Option<&LocalAvatarContext> {
        let epoch = self.context.as_ref()?.epoch.to_string();
        self.snapshot
            .content
            .as_ref()?
            .input
            .avatar
            .as_ref()
            .filter(|avatar| avatar.epoch == epoch && avatar.validate().is_ok())
    }
    pub(super) fn do_avatar_read(&mut self, req: &[u8], res: &mut [u8]) -> i32 {
        if let Err(error) = self.avatar_service("client.avatar.read") {
            return ec(error);
        }
        if !req.is_empty() {
            return ec(ErrorCode::InvalidRequest);
        }
        let avatar = self.local_avatar();
        let mut value = avatar
            .map(|avatar| serde_json::to_value(avatar).unwrap())
            .unwrap_or(serde_json::json!({"version":1}));
        value["ready"] = Value::Bool(avatar.is_some());
        let bytes = match serde_json::to_vec(&value) {
            Ok(bytes) => bytes,
            Err(_) => return ec(ErrorCode::InvalidRequest),
        };
        if bytes.len() > res.len() {
            return ec(ErrorCode::BufferTooSmall);
        }
        res[..bytes.len()].copy_from_slice(&bytes);
        bytes.len() as i32
    }
    pub(super) fn do_cape_write(&mut self, req: &[u8], _res: &mut [u8]) -> i32 {
        if let Err(error) = self.avatar_service("client.avatar.write") {
            return ec(error);
        }
        let deformation = if req.is_empty() {
            None
        } else {
            let deformation: CapeDeformation = match strict_json(req, MAX_BUF)
                .and_then(|v| serde_json::from_value(v).map_err(|e| e.to_string()))
            {
                Ok(deformation) => deformation,
                Err(_) => return ec(ErrorCode::InvalidRequest),
            };
            if deformation.validate().is_err() {
                return ec(ErrorCode::InvalidRequest);
            }
            let Some(avatar) = self.local_avatar() else {
                return ec(ErrorCode::Unavailable);
            };
            if deformation.handle != avatar.handle {
                return ec(ErrorCode::InvalidRequest);
            }
            Some(deformation)
        };
        let operation = crate::content::ContentOperation::Cape {
            epoch: self.context.as_ref().unwrap().epoch,
            deformation,
        };
        match self.stage_cmd(Command::Content {
            request: Box::new(operation),
        }) {
            Ok(()) => 0,
            Err(error) => ec(error),
        }
    }
}
