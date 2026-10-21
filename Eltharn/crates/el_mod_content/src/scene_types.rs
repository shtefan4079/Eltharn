#![forbid(unsafe_code)]
use crate::spec::{Components, InventoryStack};
use crate::{ContentId, Definition};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
pub mod avatar;
pub mod canvas;
pub mod entities;
pub mod player;
pub mod surface;
pub mod ui_input;
pub mod world;
pub mod world_region;
use canvas::{CanvasCommand, CanvasView};
use world::{BlockWatch, WorldBlocksSnapshot};
use world_region::{RegionSnapshot, RegionWatch};
const MAX_CMDS: usize = 64;
const MAX_WIDGETS: usize = 128;
const MAX_WIDGET_BYTES: usize = 32768;
const MAX_EVENTS: usize = 64;
const BLOCK_ABS: i64 = 32_000_000;
const POS_ABS: u64 = 32_000_000_000;
const ROT_ABS: u32 = 360_000;
const LOOK_ABS: i32 = 1000;
const VP_MAX: u16 = 8192;
const R_MIN: i32 = -32768;
const R_MAX: i32 = 32767;
fn id_ok(id: u32) -> Result<(), String> {
    if id == 0 {
        Err("id must be >0".into())
    } else {
        Ok(())
    }
}
fn pos_ok(p: [i64; 3]) -> Result<(), String> {
    for v in p {
        if v.unsigned_abs() > POS_ABS {
            return Err("position_milli out of range".into());
        }
    }
    Ok(())
}
fn rot_ok(r: [i32; 3]) -> Result<(), String> {
    for v in r {
        if v.unsigned_abs() > ROT_ABS {
            return Err("rotation_mdeg out of range".into());
        }
    }
    Ok(())
}
fn blk_ok(p: [i32; 3]) -> Result<(), String> {
    for v in p {
        if (v as i64).unsigned_abs() as i64 > BLOCK_ABS {
            return Err("block pos out of range".into());
        }
    }
    Ok(())
}
fn props_ok(p: &BTreeMap<String, String>) -> Result<(), String> {
    if p.len() > 64 {
        return Err("too many properties".into());
    }
    for (k, v) in p {
        if k.is_empty() || k.len() > 128 || v.len() > 1024 || k.contains('\0') || v.contains('\0') {
            return Err("bad property".into());
        }
    }
    Ok(())
}
fn comps_ok(c: &Components) -> Result<(), String> {
    if c.len() > 32 {
        return Err("too many components".into());
    }
    for m in c.values() {
        if m.len() > 32 {
            return Err("too many component fields".into());
        }
    }
    Ok(())
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectKind {
    Block,
    Entity,
    BlockEntity,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectRef {
    pub kind: ObjectKind,
    pub id: u32,
}
impl ObjectRef {
    pub fn validate_local(&self) -> Result<(), String> {
        id_ok(self.id)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObjectView {
    pub object: ObjectRef,
    pub content: ContentId,
    pub position_milli: [i64; 3],
    pub rotation_mdeg: [i32; 3],
    pub properties: BTreeMap<String, String>,
    pub components: Components,
    pub anchor: Option<u32>,
}
impl ObjectView {
    pub fn validate_local(&self) -> Result<(), String> {
        self.object.validate_local()?;
        pos_ok(self.position_milli)?;
        rot_ok(self.rotation_mdeg)?;
        props_ok(&self.properties)?;
        comps_ok(&self.components)?;
        match (self.object.kind, self.anchor) {
            (ObjectKind::BlockEntity, Some(a)) => id_ok(a),
            (ObjectKind::BlockEntity, None) => Err("block_entity needs anchor".into()),
            (_, Some(_)) => Err("anchor only for block_entity".into()),
            _ => Ok(()),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Widget {
    Label {
        id: u32,
        rect: [i32; 4],
        text: String,
        color: u32,
    },
    Button {
        id: u32,
        rect: [i32; 4],
        text: String,
        color: u32,
    },
    Image {
        id: u32,
        rect: [i32; 4],
        texture: ContentId,
    },
    Canvas {
        id: u32,
        rect: [i32; 4],
        canvas: u32,
    },
    Slot {
        id: u32,
        rect: [i32; 4],
        inventory: u32,
        slot: u16,
    },
}
fn rect_ok(r: [i32; 4]) -> Result<(), String> {
    if r[0] < R_MIN || r[0] > R_MAX || r[1] < R_MIN || r[1] > R_MAX {
        return Err("rect xy out of range".into());
    }
    if r[2] < 1 || r[2] > 4096 || r[3] < 1 || r[3] > 4096 {
        return Err("rect size out of range".into());
    }
    Ok(())
}
fn text_ok(s: &str) -> Result<(), String> {
    if s.len() > 1024 || s.contains('\0') {
        return Err("bad text".into());
    }
    Ok(())
}
impl Widget {
    pub fn id(&self) -> u32 {
        match *self {
            Self::Label { id, .. }
            | Self::Button { id, .. }
            | Self::Image { id, .. }
            | Self::Canvas { id, .. }
            | Self::Slot { id, .. } => id,
        }
    }
    pub fn rect(&self) -> [i32; 4] {
        match *self {
            Self::Label { rect, .. }
            | Self::Button { rect, .. }
            | Self::Image { rect, .. }
            | Self::Canvas { rect, .. }
            | Self::Slot { rect, .. } => rect,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        id_ok(self.id())?;
        rect_ok(self.rect())?;
        match self {
            Self::Label { text, .. } | Self::Button { text, .. } => text_ok(text),
            Self::Image { .. } => Ok(()),
            Self::Canvas { canvas, .. } => id_ok(*canvas),
            Self::Slot { inventory, .. } => id_ok(*inventory),
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiView {
    pub screen: bool,
    pub widgets: Vec<Widget>,
}
impl UiView {
    pub fn validate_local(&self) -> Result<(), String> {
        if self.widgets.len() > MAX_WIDGETS {
            return Err("too many widgets".into());
        }
        let mut seen = BTreeSet::new();
        for w in &self.widgets {
            w.validate()?;
            if !seen.insert(w.id()) {
                return Err("duplicate widget id".into());
            }
        }
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_WIDGET_BYTES {
            return Err("widgets too large".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputEvent {
    pub sequence: u64,
    pub kind: String,
    pub target: Option<ObjectRef>,
    pub widget: Option<u32>,
    pub key: Option<String>,
    pub pressed: bool,
    pub pointer: [i32; 2],
}
impl InputEvent {
    pub fn validate_local(&self) -> Result<(), String> {
        if self.sequence == 0 {
            return Err("event sequence must be >0".into());
        }
        match self.kind.as_str() {
            "key" | "pointer" | "activate" | "select" | "close" => {}
            _ => return Err("bad input kind".into()),
        }
        if let Some(t) = &self.target {
            t.validate_local()?;
        }
        if let Some(w) = self.widget {
            id_ok(w)?;
        }
        if let Some(k) = &self.key {
            if k.is_empty() || k.len() > 48 || !k.bytes().all(|b| b >= 0x20 && b <= 0x7E) {
                return Err("bad key".into());
            }
        }
        for v in self.pointer {
            if v < R_MIN || v > R_MAX {
                return Err("pointer out of range".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputFrame {
    pub sequence: u64,
    pub camera_milli: [i64; 3],
    pub look_milli: [i32; 3],
    pub viewport: [u16; 2],
    pub events: Vec<InputEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_context: Option<ui_input::UiInputContext>,
    #[serde(default)]
    pub channels: Vec<ChannelInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_blocks: Option<WorldBlocksSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_region: Option<RegionSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_surface: Option<surface::SurfaceSnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player: Option<player::PlayerContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<avatar::LocalAvatarContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entities: Option<entities::EntitySnapshot>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelInput {
    pub sender: String,
    pub channel: ContentId,
    pub data: Vec<u8>,
}

impl ChannelInput {
    pub fn validate_local(&self) -> Result<(), String> {
        let (package, slot) = self.sender.rsplit_once('/').ok_or("bad channel sender")?;
        let package: el_extension_api::ExtensionId =
            package.parse().map_err(|_| "bad channel sender package")?;
        if !matches!(slot, "server" | "shared")
            || self.sender.len() > 160
            || self.channel.is_reserved_namespace()
            || self.channel.namespace() != package.namespace()
            || self.data.len() > 8192
        {
            return Err("invalid channel input".into());
        }
        Ok(())
    }
}
impl InputFrame {
    pub fn validate(&self) -> Result<(), String> {
        if let Some(entities) = &self.entities {
            entities.validate()?;
        }
        if let Some(context) = &self.ui_context {
            context.validate()?;
        }
        if let Some(avatar) = &self.avatar {
            avatar.validate()?;
        }
        if let Some(snapshot) = &self.world_surface {
            snapshot.validate()?;
        }
        if let Some(player) = &self.player {
            player.validate()?;
        }
        if let Some(snapshot) = &self.world_blocks {
            snapshot.validate()?;
        }
        if let Some(snapshot) = &self.world_region {
            snapshot.validate()?;
        }
        pos_ok(self.camera_milli)?;
        for v in self.look_milli {
            if v.unsigned_abs() > LOOK_ABS as u32 {
                return Err("look out of range".into());
            }
        }
        for v in self.viewport {
            if v > VP_MAX {
                return Err("viewport out of range".into());
            }
        }
        if self.events.len() > MAX_EVENTS {
            return Err("too many input events".into());
        }
        let mut last = 0u64;
        for e in &self.events {
            e.validate_local()?;
            if e.sequence <= last {
                return Err("input sequences must increase".into());
            }
            last = e.sequence;
        }
        if self.channels.len() > 32 {
            return Err("too many channel inputs".into());
        }
        for message in &self.channels {
            message.validate_local()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryView {
    pub id: u32,
    pub slots: Vec<Option<InventoryStack>>,
}
impl InventoryView {
    pub fn validate_local(&self) -> Result<(), String> {
        id_ok(self.id)?;
        if self.slots.is_empty() || self.slots.len() > 64 {
            return Err("bad slot count".into());
        }
        for s in &self.slots {
            if let Some(st) = s {
                st.validate().map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorView {
    pub owner: String,
    pub objects: Vec<ObjectView>,
    pub inventories: Vec<InventoryView>,
    pub ui: UiView,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub camera: Option<CameraSettings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cape: Option<avatar::CapeDeformation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_watch: Option<BlockWatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_region: Option<RegionWatch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub world_surface: Option<surface::SurfaceWatch>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub canvases: Vec<CanvasView>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraSettings {
    pub hurt_scale_milli: u16,
}
impl CameraSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.hurt_scale_milli > 1000 {
            Err("camera hurt_scale_milli must be in 0..=1000".into())
        } else {
            Ok(())
        }
    }
}
impl ActorView {
    pub fn validate_local(&self) -> Result<(), String> {
        if let Some(cape) = &self.cape {
            cape.validate()?;
        }
        if let Some(watch) = &self.world_surface {
            watch.validate()?;
        }
        if usize::from(self.world_watch.is_some())
            + usize::from(self.world_region.is_some())
            + usize::from(self.world_surface.is_some())
            > 1
        {
            return Err("one world subscription per actor".into());
        }
        if self.canvases.len() > 1 {
            return Err("one canvas per actor".into());
        }
        for canvas in &self.canvases {
            canvas.validate()?;
        }
        for widget in &self.ui.widgets {
            if let Widget::Canvas { canvas, .. } = widget {
                if !self.canvases.iter().any(|c| c.id == *canvas) {
                    return Err("widget canvas is missing".into());
                }
            }
        }
        if let Some(watch) = &self.world_watch {
            watch.validate()?;
        }
        if let Some(watch) = &self.world_region {
            watch.validate()?;
        }
        if let Some(camera) = self.camera {
            camera.validate()?;
        }
        if self.owner.is_empty() || self.owner.len() > 160 || self.owner.contains('\0') {
            return Err("bad owner".into());
        }
        if self.objects.len() > 128 {
            return Err("too many objects".into());
        }
        if self.inventories.len() > 8 {
            return Err("too many inventories".into());
        }
        let mut oseen = BTreeSet::new();
        for o in &self.objects {
            o.validate_local()?;
            if !oseen.insert((o.object.kind, o.object.id)) {
                return Err("duplicate object".into());
            }
        }
        let mut iseen = BTreeSet::new();
        for i in &self.inventories {
            i.validate_local()?;
            if !iseen.insert(i.id) {
                return Err("duplicate inventory".into());
            }
        }
        self.ui.validate_local()
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneView {
    pub epoch: u64,
    pub revision: u64,
    pub actors: Vec<ActorView>,
    pub sounds: Vec<crate::scene::SoundCue>,
}
impl SceneView {
    pub fn validate_local(&self) -> Result<(), String> {
        if self.actors.len() > 32 {
            return Err("too many actors".into());
        }
        let mut seen = BTreeSet::new();
        for a in &self.actors {
            a.validate_local()?;
            if a.cape
                .as_ref()
                .is_some_and(|cape| cape.identity().is_none_or(|(_, epoch)| epoch != self.epoch))
            {
                return Err("cape belongs to another scene epoch".into());
            }
            if a.canvases.iter().any(|c| c.rgba_base64.is_none()) {
                return Err("scene canvas pixels missing".into());
            }
            if !seen.insert(&a.owner) {
                return Err("duplicate actor".into());
            }
        }
        if self.sounds.len() > 64 {
            return Err("too many sound cues".into());
        }
        let mut last = 0;
        for cue in &self.sounds {
            if cue.sequence <= last || !seen.contains(&cue.owner) {
                return Err("invalid sound cue sequence or owner".into());
            }
            pos_ok(cue.position_milli)?;
            if cue.volume_milli > 4000 || !(250..=4000).contains(&cue.pitch_milli) {
                return Err("invalid sound cue volume/pitch".into());
            }
            last = cue.sequence;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorSnapshot {
    pub epoch: u64,
    pub revision: u64,
    #[serde(default)]
    pub ui_revisions: ui_input::UiTargetRevisions,
    pub actor: ActorView,
}
impl ActorSnapshot {
    pub fn validate_local(&self) -> Result<(), String> {
        self.ui_revisions.validate()?;
        self.actor.validate_local()?;
        if self
            .actor
            .cape
            .as_ref()
            .is_some_and(|cape| cape.identity().is_none_or(|(_, epoch)| epoch != self.epoch))
        {
            return Err("cape belongs to another actor snapshot epoch".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneCommand {
    Cape {
        deformation: Option<avatar::CapeDeformation>,
    },
    Camera {
        settings: Option<CameraSettings>,
    },
    WorldWatch {
        watch: BlockWatch,
    },
    WorldRegion {
        watch: RegionWatch,
    },
    WorldSurface {
        watch: surface::SurfaceWatch,
    },
    Canvas {
        command: CanvasCommand,
    },
    PutBlock {
        object: u32,
        block: ContentId,
        position: [i32; 3],
        properties: BTreeMap<String, String>,
    },
    RemoveBlock {
        object: u32,
    },
    SpawnEntity {
        object: u32,
        entity: ContentId,
        position_milli: [i64; 3],
    },
    MoveEntity {
        object: u32,
        position_milli: [i64; 3],
        rotation_mdeg: [i32; 3],
    },
    RemoveEntity {
        object: u32,
    },
    CreateBlockEntity {
        object: u32,
        block_entity: ContentId,
        block_object: u32,
    },
    RemoveBlockEntity {
        object: u32,
    },
    SetComponents {
        target: ObjectRef,
        components: Components,
    },
    CreateInventory {
        inventory: u32,
        slots: u16,
    },
    SetSlot {
        inventory: u32,
        slot: u16,
        stack: Option<InventoryStack>,
    },
    Transfer {
        inventory: u32,
        from: u16,
        to: u16,
        count: u16,
    },
    Craft {
        inventory: u32,
        recipe: ContentId,
        components: Components,
    },
    Loot {
        inventory: u32,
        table: ContentId,
        roll: u64,
        components: Components,
    },
    Ui {
        view: UiView,
    },
    CloseUi,
    PlaySound {
        sound: ContentId,
        position_milli: [i64; 3],
        volume_milli: u16,
        pitch_milli: u16,
    },
}
impl SceneCommand {
    pub fn validate_local(&self) -> Result<(), String> {
        match self {
            Self::Cape { deformation } => deformation.as_ref().map_or(Ok(()), |d| d.validate()),
            Self::Camera { settings } => settings.map_or(Ok(()), |s| s.validate()),
            Self::WorldWatch { watch } => watch.validate(),
            Self::WorldRegion { watch } => watch.validate(),
            Self::WorldSurface { watch } => watch.validate(),
            Self::Canvas { command } => command.validate(),
            Self::PutBlock {
                object,
                position,
                properties,
                ..
            } => {
                id_ok(*object)?;
                blk_ok(*position)?;
                props_ok(properties)
            }
            Self::RemoveBlock { object }
            | Self::RemoveEntity { object }
            | Self::RemoveBlockEntity { object } => id_ok(*object),
            Self::SpawnEntity {
                object,
                position_milli,
                ..
            } => {
                id_ok(*object)?;
                pos_ok(*position_milli)
            }
            Self::MoveEntity {
                object,
                position_milli,
                rotation_mdeg,
            } => {
                id_ok(*object)?;
                pos_ok(*position_milli)?;
                rot_ok(*rotation_mdeg)
            }
            Self::CreateBlockEntity {
                object,
                block_object,
                ..
            } => {
                id_ok(*object)?;
                id_ok(*block_object)
            }
            Self::SetComponents { target, components } => {
                target.validate_local()?;
                comps_ok(components)
            }
            Self::CreateInventory { inventory, slots } => {
                id_ok(*inventory)?;
                if *slots < 1 || *slots > 64 {
                    return Err("bad slot count".into());
                }
                Ok(())
            }
            Self::SetSlot {
                inventory, stack, ..
            } => {
                id_ok(*inventory)?;
                if let Some(s) = stack {
                    s.validate().map_err(|e| e.to_string())?;
                }
                Ok(())
            }
            Self::Transfer {
                inventory,
                from,
                to,
                count,
            } => {
                id_ok(*inventory)?;
                if from == to {
                    return Err("transfer needs distinct slots".into());
                }
                if *count == 0 {
                    return Err("transfer count must be >0".into());
                }
                Ok(())
            }
            Self::Craft {
                inventory,
                components,
                ..
            }
            | Self::Loot {
                inventory,
                components,
                ..
            } => {
                id_ok(*inventory)?;
                comps_ok(components)
            }
            Self::Ui { view } => view.validate_local(),
            Self::CloseUi => Ok(()),
            Self::PlaySound {
                position_milli,
                volume_milli,
                pitch_milli,
                ..
            } => {
                pos_ok(*position_milli)?;
                if *volume_milli > 4000 {
                    return Err("volume out of range".into());
                }
                if *pitch_milli < 250 || *pitch_milli > 4000 {
                    return Err("pitch out of range".into());
                }
                Ok(())
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneRequest {
    pub epoch: u64,
    pub commands: Vec<SceneCommand>,
}
impl SceneRequest {
    pub fn validate_local(&self) -> Result<(), String> {
        if self.commands.len() > MAX_CMDS {
            return Err("too many commands".into());
        }
        for c in &self.commands {
            c.validate_local()?;
        }
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        Definition::parse(&bytes)
            .map(|_| ())
            .map_err(|e| format!("bad request bytes: {e}"))
    }
}
