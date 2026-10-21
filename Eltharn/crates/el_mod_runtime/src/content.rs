use std::collections::BTreeMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::lifecycle::Phase;
use el_extension_api::ExtensionId;
use el_mod_content::Definition;
use el_mod_content::catalog::{Catalog, CatalogBuilder, OwnedSpec};
use el_mod_content::scene::{SceneCheckpoint, SceneState};
use el_mod_content::scene_types::InputFrame;
use el_mod_content::scene_types::canvas::CanvasCommand;
use el_mod_content::scene_types::world::BlockWatch;
use el_mod_content::scene_types::world_region::RegionWatch;
use el_mod_content::scene_types::{
    ActorSnapshot, CameraSettings, SceneCommand, SceneRequest, SceneView,
};
use el_mod_content::spec::ContentSpec;

pub const MAX_OP_BYTES: usize = 64 * 1024;
pub const MAX_SNAPSHOT_BYTES: usize = 64 * 1024;
pub const MAX_INPUT_BYTES: usize = 64 * 1024;
const MAX_REGISTER_DEFS: usize = 64;
pub const MAX_INPUT_OWNERS: usize = 16;
pub const MAX_OWNER_LEN: usize = 160;

fn check_json_bytes(bytes: &[u8], max: usize, what: &str) -> Result<(), String> {
    if bytes.len() > max {
        return Err(format!("{what} too large: {}", bytes.len()));
    }
    Definition::parse(bytes)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn parse_owner(owner: &str) -> Result<ExtensionId, String> {
    if owner.is_empty() || owner.len() > MAX_OWNER_LEN || owner.contains('\0') {
        return Err("bad owner".into());
    }
    let (pkg, slot) = owner
        .rsplit_once('/')
        .ok_or_else(|| "bad owner (need package/slot)".to_string())?;
    if pkg.is_empty() {
        return Err("bad owner package".into());
    }
    if slot.is_empty() || slot.contains('/') || slot.contains('\0') {
        return Err("bad owner slot".into());
    }
    pkg.parse::<ExtensionId>()
        .map_err(|e| format!("bad owner id: {e:?}"))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContentOperation {
    Cape {
        epoch: u64,
        deformation: Option<el_mod_content::scene_types::avatar::CapeDeformation>,
    },
    Register {
        definitions: Vec<ContentSpec>,
    },
    Apply {
        request: SceneRequest,
    },
    Camera {
        epoch: u64,
        settings: Option<CameraSettings>,
    },
    WorldWatch {
        epoch: u64,
        watch: BlockWatch,
    },
    WorldRegion {
        epoch: u64,
        watch: RegionWatch,
    },
    WorldSurface {
        epoch: u64,
        watch: el_mod_content::scene_types::surface::SurfaceWatch,
    },
    Canvas {
        epoch: u64,
        command: CanvasCommand,
    },
}

impl ContentOperation {
    pub fn capability(&self) -> &'static str {
        match self {
            Self::Cape { .. } => "client.avatar.write",
            Self::Register { .. } => "content.register",
            Self::Apply { .. } => "content.client.write",
            Self::Camera { .. } => "client.camera.write",
            Self::WorldWatch { .. } => "world.blocks.read",
            Self::WorldRegion { .. } => "world.blocks.read",
            Self::WorldSurface { .. } => "world.blocks.read",
            Self::Canvas { .. } => "client.canvas.write",
        }
    }

    pub fn capability_for_side(&self, side: u32) -> &'static str {
        match self {
            Self::Apply { .. } if side == 2 => "content.server.write",
            _ => self.capability(),
        }
    }

    pub fn validate(&self, phase: Phase) -> Result<(), String> {
        match self {
            Self::Cape { epoch, deformation } => {
                if !matches!(phase, Phase::WorldEnter | Phase::Tick) {
                    return Err("cape only in WorldEnter|Tick".into());
                }
                if let Some(deformation) = deformation {
                    deformation.validate()?;
                    if deformation.identity().is_none_or(|(_, e)| e != *epoch) {
                        return Err("stale cape epoch".into());
                    }
                }
                Ok(())
            }
            Self::Register { definitions } => {
                if phase != Phase::Init {
                    return Err("register only in Init".into());
                }
                if definitions.len() > MAX_REGISTER_DEFS {
                    return Err("too many definitions".into());
                }
                for d in definitions {
                    d.validate_local()?;
                }
                let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
                check_json_bytes(&bytes, MAX_OP_BYTES, "operation")?;
                Ok(())
            }
            Self::Apply { request } => {
                if phase != Phase::WorldEnter && phase != Phase::Tick {
                    return Err("apply only in WorldEnter|Tick".into());
                }
                request.validate_local()?;
                if request.commands.iter().any(|c| {
                    matches!(
                        c,
                        SceneCommand::Camera { .. }
                            | SceneCommand::Cape { .. }
                            | SceneCommand::WorldWatch { .. }
                            | SceneCommand::WorldRegion { .. }
                            | SceneCommand::WorldSurface { .. }
                            | SceneCommand::Canvas { .. }
                    )
                }) {
                    return Err("client services require their dedicated capabilities".into());
                }
                let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
                check_json_bytes(&bytes, MAX_OP_BYTES, "operation")?;
                Ok(())
            }
            Self::Camera { settings, .. } => {
                if !matches!(phase, Phase::WorldEnter | Phase::Tick) {
                    return Err("camera only in WorldEnter|Tick".into());
                }
                settings.map_or(Ok(()), |s| s.validate())
            }
            Self::WorldWatch { watch, .. } => {
                if !matches!(phase, Phase::WorldEnter | Phase::Tick) {
                    return Err("world subscription only in WorldEnter|Tick".into());
                }
                watch.validate()
            }
            Self::WorldRegion { watch, .. } => {
                if !matches!(phase, Phase::WorldEnter | Phase::Tick) {
                    return Err("world region only in WorldEnter|Tick".into());
                }
                watch.validate()
            }
            Self::WorldSurface { watch, .. } => {
                if !matches!(phase, Phase::WorldEnter | Phase::Tick) {
                    return Err("surface only in WorldEnter|Tick".into());
                }
                watch.validate()
            }
            Self::Canvas { command, .. } => {
                if !matches!(phase, Phase::WorldEnter | Phase::Tick) {
                    return Err("canvas only inWorldEnter|Tick".into());
                }
                command.validate()
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContentSnapshot {
    pub scene: ActorSnapshot,
    pub input: InputFrame,
}

impl ContentSnapshot {
    pub fn validate(&self) -> Result<(), String> {
        self.scene.validate_local()?;
        self.input.validate()?;
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err("snapshot too large".into());
        }
        let mut integer_view = serde_json::to_value(self).map_err(|e| e.to_string())?;
        integer_view["input"]
            .as_object_mut()
            .unwrap()
            .remove("player");
        integer_view["input"]
            .as_object_mut()
            .unwrap()
            .remove("avatar");
        integer_view["input"]
            .as_object_mut()
            .unwrap()
            .remove("entities");
        check_json_bytes(
            &serde_json::to_vec(&integer_view).map_err(|e| e.to_string())?,
            MAX_SNAPSHOT_BYTES,
            "snapshot",
        )?;
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ContentReport {
    pub readers: Vec<String>,
    pub fingerprint: Option<String>,
    pub catalog: Option<Vec<OwnedSpec>>,
    pub scene: SceneView,
    pub ui_revisions: BTreeMap<String, el_mod_content::scene_types::ui_input::UiTargetRevisions>,
}

#[derive(Clone, Default)]
pub struct ContentEngine {
    builder: CatalogBuilder,
    catalog: Option<Arc<Catalog>>,
    scene: SceneState,
    input: BTreeMap<String, InputFrame>,
    seen_input: BTreeMap<String, (u64, u64)>,
}

impl ContentEngine {
    pub fn has_catalog(&self) -> bool {
        self.catalog.is_some()
    }

    pub fn apply(
        &mut self,
        owner: &str,
        phase: Phase,
        op: &ContentOperation,
    ) -> Result<(), String> {
        op.validate(phase)?;
        let owner_id = parse_owner(owner)?;
        match op {
            ContentOperation::Canvas { epoch, command } => {
                let catalog = self.catalog.as_ref().ok_or("catalog not frozen")?;
                self.scene.apply(
                    catalog,
                    owner,
                    &SceneRequest {
                        epoch: *epoch,
                        commands: vec![SceneCommand::Canvas {
                            command: command.clone(),
                        }],
                    },
                )
            }
            ContentOperation::WorldSurface { epoch, watch } => {
                let catalog = self.catalog.as_ref().ok_or("catalog not frozen")?;
                self.scene.apply(
                    catalog,
                    owner,
                    &SceneRequest {
                        epoch: *epoch,
                        commands: vec![SceneCommand::WorldSurface {
                            watch: watch.clone(),
                        }],
                    },
                )?;
                if let Some(input) = self.input.get_mut(owner) {
                    input.world_blocks = None;
                    input.world_region = None;
                    if input
                        .world_surface
                        .as_ref()
                        .is_some_and(|s| s.watch != *watch)
                        || watch.columns() == 0
                    {
                        input.world_surface = None;
                    }
                }
                Ok(())
            }
            ContentOperation::WorldRegion { epoch, watch } => {
                let catalog = self.catalog.as_ref().ok_or("catalog not frozen")?;
                self.scene.apply(
                    catalog,
                    owner,
                    &SceneRequest {
                        epoch: *epoch,
                        commands: vec![SceneCommand::WorldRegion {
                            watch: watch.clone(),
                        }],
                    },
                )?;
                if let Some(input) = self.input.get_mut(owner) {
                    input.world_blocks = None;
                    input.world_region = None;
                    input.world_surface = None;
                }
                Ok(())
            }
            ContentOperation::WorldWatch { epoch, watch } => {
                let catalog = self.catalog.as_ref().ok_or("catalog not frozen")?;
                self.scene.apply(
                    catalog,
                    owner,
                    &SceneRequest {
                        epoch: *epoch,
                        commands: vec![SceneCommand::WorldWatch {
                            watch: watch.clone(),
                        }],
                    },
                )?;
                if let Some(input) = self.input.get_mut(owner) {
                    input.world_blocks = None;
                    input.world_region = None;
                    input.world_surface = None;
                }
                Ok(())
            }
            ContentOperation::Camera { epoch, settings } => {
                let catalog = self.catalog.as_ref().ok_or("catalog not frozen")?;
                self.scene.apply(
                    catalog,
                    owner,
                    &SceneRequest {
                        epoch: *epoch,
                        commands: vec![SceneCommand::Camera {
                            settings: *settings,
                        }],
                    },
                )
            }
            ContentOperation::Cape { epoch, deformation } => {
                if let Some(deformation) = deformation {
                    let avatar = self
                        .input
                        .get(owner)
                        .and_then(|frame| frame.avatar.as_ref())
                        .ok_or("local avatar input unavailable")?;
                    avatar.validate()?;
                    if deformation.handle != avatar.handle || avatar.epoch != epoch.to_string() {
                        return Err("cape handle differs from current avatar input".into());
                    }
                }
                let catalog = self.catalog.as_ref().ok_or("catalog not frozen")?;
                self.scene.apply(
                    catalog,
                    owner,
                    &SceneRequest {
                        epoch: *epoch,
                        commands: vec![SceneCommand::Cape {
                            deformation: deformation.clone(),
                        }],
                    },
                )
            }
            ContentOperation::Register { definitions } => {
                if self.catalog.is_some() {
                    return Err("catalog already frozen".into());
                }
                self.builder.register_batch(owner_id, definitions.clone())?;
                Ok(())
            }
            ContentOperation::Apply { request } => {
                let cat = self.catalog.as_ref().ok_or("catalog not frozen")?;
                self.scene.apply(cat, owner, request)?;
                Ok(())
            }
        }
    }

    pub fn freeze(&mut self) -> Result<(), String> {
        if self.catalog.is_some() {
            return Err("already frozen".into());
        }
        let cat = self.builder.clone().freeze()?;
        self.catalog = Some(Arc::new(cat));
        self.builder = CatalogBuilder::default();
        Ok(())
    }

    pub fn enter(&mut self, epoch: u64) -> Result<(), String> {
        let _ = self.catalog.as_ref().ok_or("catalog not frozen")?;
        self.scene.enter(epoch)?;
        self.input.clear();
        self.seen_input.clear();
        Ok(())
    }

    pub fn exit(&mut self) {
        self.scene.exit();
        self.input.clear();
        self.seen_input.clear();
    }

    pub fn shutdown(&mut self) {
        self.exit();
        self.builder = CatalogBuilder::default();
        self.catalog = None;
    }

    pub fn disable(&mut self, owner: &str) {
        self.scene.disable(owner);
        self.input.remove(owner);
        self.seen_input.remove(owner);
    }

    pub fn snapshot(&self, owner: &str) -> ContentSnapshot {
        let mut snapshot = ContentSnapshot {
            scene: self.scene.snapshot(owner),
            input: self.input.get(owner).cloned().unwrap_or_default(),
        };
        for canvas in &mut snapshot.scene.actor.canvases {
            canvas.rgba_base64 = None;
        }
        if snapshot.input.world_region.is_some()
            && serde_json::to_vec(&snapshot).map_or(true, |bytes| bytes.len() > MAX_SNAPSHOT_BYTES)
        {
            if let Some(region) = snapshot.input.world_region.as_mut() {
                region.status =
                    el_mod_content::scene_types::world_region::RegionStatus::BudgetLimit;
                region.palette.clear();
                region.indices.clear();
            }
        }
        if snapshot.input.world_surface.is_some()
            && serde_json::to_vec(&snapshot).map_or(true, |bytes| bytes.len() > MAX_SNAPSHOT_BYTES)
        {
            snapshot
                .input
                .world_surface
                .as_mut()
                .unwrap()
                .budget_limit();
        }
        snapshot
    }

    pub fn report(&self, include_catalog: bool) -> ContentReport {
        ContentReport {
            readers: Vec::new(),
            fingerprint: self.catalog.as_ref().map(|c| c.fingerprint().to_owned()),
            catalog: if include_catalog {
                self.catalog.as_ref().map(|c| c.export())
            } else {
                None
            },
            scene: self.scene.view(),
            ui_revisions: self.scene.ui_revisions(),
        }
    }

    pub fn checkpoint_scene(&self) -> Result<SceneCheckpoint, String> {
        self.scene.checkpoint()
    }

    pub fn restore_scene(
        &mut self,
        checkpoint: &SceneCheckpoint,
        epoch: u64,
    ) -> Result<(), String> {
        let catalog = self.catalog.as_ref().ok_or("catalog not frozen")?;
        let restored = SceneState::restore(catalog, checkpoint, epoch)?;
        self.scene = restored;
        Ok(())
    }

    pub fn set_input(&mut self, mut frames: BTreeMap<String, InputFrame>) -> Result<(), String> {
        if frames.len() > MAX_INPUT_OWNERS {
            return Err("too many input owners".into());
        }
        let mut seen = self.seen_input.clone();
        for (owner, frame) in &frames {
            if owner.is_empty() || owner.len() > MAX_OWNER_LEN || owner.contains('\0') {
                return Err("bad input owner".into());
            }
            frame.validate()?;
            let mut integer_view = serde_json::to_value(frame).map_err(|e| e.to_string())?;
            if serde_json::to_vec(&integer_view)
                .map_err(|e| e.to_string())?
                .len()
                > MAX_INPUT_BYTES
            {
                return Err("input too large".into());
            }
            integer_view.as_object_mut().unwrap().remove("player");
            integer_view.as_object_mut().unwrap().remove("avatar");
            integer_view.as_object_mut().unwrap().remove("entities");
            check_json_bytes(
                &serde_json::to_vec(&integer_view).map_err(|e| e.to_string())?,
                MAX_INPUT_BYTES,
                "input",
            )?;
            let (previous_frame, previous_event) = seen.get(owner).copied().unwrap_or_default();
            if (frame.sequence == 0
                && (!frame.events.is_empty() || !frame.channels.is_empty() || previous_frame != 0))
                || (frame.sequence != 0 && frame.sequence <= previous_frame)
            {
                return Err("input frame sequence must increase".into());
            }
            if frame
                .events
                .first()
                .is_some_and(|e| e.sequence <= previous_event)
            {
                return Err("input event sequence must increase".into());
            }
            seen.insert(
                owner.clone(),
                (
                    frame.sequence,
                    frame
                        .events
                        .last()
                        .map(|e| e.sequence)
                        .unwrap_or(previous_event),
                ),
            );
        }
        if seen.len() > MAX_INPUT_OWNERS {
            return Err("too many input owners".into());
        }
        for (owner, frame) in &mut frames {
            if let Some(context) = &frame.ui_context {
                let snapshot = self.scene.snapshot(owner);
                frame
                    .events
                    .retain(|event| context.permits(snapshot.epoch, &snapshot.ui_revisions, event));
            }
        }
        self.input = frames;
        self.seen_input = seen;
        Ok(())
    }

    pub fn finish_tick(&mut self) {
        for frame in self.input.values_mut() {
            frame.events.clear();
            frame.channels.clear();
        }
    }
}
