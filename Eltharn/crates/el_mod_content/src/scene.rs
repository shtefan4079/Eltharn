#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ContentId;
use crate::catalog::Catalog;
use crate::data::Inventory;
use crate::scene_types::{
    ActorSnapshot, ActorView, InventoryView, ObjectKind, ObjectRef, ObjectView, SceneCommand,
    SceneRequest, SceneView, UiView, Widget,
};
use crate::spec::{ContentSpec, Kind};

mod checkpoint;
pub use checkpoint::{ActorCheckpoint, SceneCheckpoint};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundCue {
    pub sequence: u64,
    pub owner: String,
    pub sound: ContentId,
    pub position_milli: [i64; 3],
    pub volume_milli: u16,
    pub pitch_milli: u16,
}

#[derive(Clone, Debug)]
struct ActorState {
    view: ActorView,
    ui_revisions: crate::scene_types::ui_input::UiTargetRevisions,
    inventories: BTreeMap<u32, Inventory>,
    high_water: BTreeMap<ObjectKind, u32>,
    last_inventory: u32,
}

#[derive(Clone, Debug, Default)]
pub struct SceneState {
    active: bool,
    epoch: u64,
    revision: u64,
    ui_clock: u64,
    actors: BTreeMap<String, ActorState>,
    sounds: Vec<SoundCue>,
    last_sound: u64,
    fingerprint: Option<String>,
}

fn owner_ok(owner: &str) -> Result<(), String> {
    if owner.is_empty() || owner.len() > 160 || owner.contains('\0') {
        return Err("bad owner".into());
    }
    Ok(())
}

fn build_actor_view(state: &ActorState) -> ActorView {
    let mut inventories = Vec::with_capacity(state.inventories.len());
    for (id, inv) in &state.inventories {
        inventories.push(InventoryView {
            id: *id,
            slots: inv.slots().to_vec(),
        });
    }
    ActorView {
        owner: state.view.owner.clone(),
        objects: state.view.objects.clone(),
        inventories,
        ui: state.view.ui.clone(),
        camera: state.view.camera,
        cape: state.view.cape.clone(),
        world_watch: state.view.world_watch.clone(),
        world_region: state.view.world_region.clone(),
        world_surface: state.view.world_surface.clone(),
        canvases: state.view.canvases.clone(),
    }
}

fn block_milli(p: [i32; 3]) -> [i64; 3] {
    [p[0] as i64 * 1000, p[1] as i64 * 1000, p[2] as i64 * 1000]
}

impl SceneState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter(&mut self, epoch: u64) -> Result<(), String> {
        if epoch <= self.epoch {
            return Err("bad epoch".into());
        }
        let revision = self.revision.checked_add(1).ok_or("revision overflow")?;
        self.actors.clear();
        self.sounds.clear();
        self.fingerprint = None;
        self.last_sound = 0;
        self.ui_clock = 0;
        self.epoch = epoch;
        self.active = true;
        self.revision = revision;
        Ok(())
    }

    pub fn exit(&mut self) {
        self.actors.clear();
        self.sounds.clear();
        self.last_sound = 0;
        self.active = false;
        self.revision = self.revision.saturating_add(1);
    }

    pub fn disable(&mut self, owner: &str) {
        if self.actors.remove(owner).is_some() {
            self.sounds.retain(|c| c.owner != owner);
            self.revision = self.revision.saturating_add(1);
        }
    }

    pub fn view(&self) -> SceneView {
        let mut actors = Vec::with_capacity(self.actors.len());
        for state in self.actors.values() {
            actors.push(build_actor_view(state));
        }
        SceneView {
            epoch: self.epoch,
            revision: self.revision,
            actors,
            sounds: self.sounds.clone(),
        }
    }

    pub fn snapshot(&self, owner: &str) -> ActorSnapshot {
        match self.actors.get(owner) {
            Some(state) => ActorSnapshot {
                epoch: self.epoch,
                revision: self.revision,
                ui_revisions: state.ui_revisions.clone(),
                actor: build_actor_view(state),
            },
            None => ActorSnapshot {
                epoch: self.epoch,
                revision: self.revision,
                ui_revisions: Default::default(),
                actor: ActorView {
                    owner: owner.to_owned(),
                    objects: Vec::new(),
                    inventories: Vec::new(),
                    ui: UiView::default(),
                    camera: None,
                    cape: None,
                    world_watch: None,
                    world_region: None,
                    world_surface: None,
                    canvases: vec![],
                },
            },
        }
    }

    pub fn sounds(&self) -> &[SoundCue] {
        &self.sounds
    }

    pub fn ui_revisions(
        &self,
    ) -> BTreeMap<String, crate::scene_types::ui_input::UiTargetRevisions> {
        self.actors
            .iter()
            .map(|(owner, state)| (owner.clone(), state.ui_revisions.clone()))
            .collect()
    }

    pub fn apply(
        &mut self,
        catalog: &Catalog,
        owner: &str,
        request: &SceneRequest,
    ) -> Result<(), String> {
        if !self.active {
            return Err("scene not active".into());
        }
        if request.epoch != self.epoch {
            return Err("bad epoch".into());
        }
        owner_ok(owner)?;
        if self
            .fingerprint
            .as_deref()
            .is_some_and(|f| f != catalog.fingerprint())
        {
            return Err("scene belongs to another catalog snapshot".into());
        }
        request.validate_local()?;
        if request.commands.is_empty() {
            return Ok(());
        }
        let mut staged: ActorState = match self.actors.get(owner) {
            Some(existing) => existing.clone(),
            None => {
                if self.actors.len() >= 16 {
                    return Err("too many actors".into());
                }
                ActorState {
                    ui_revisions: Default::default(),
                    view: ActorView {
                        owner: owner.to_owned(),
                        objects: Vec::new(),
                        inventories: Vec::new(),
                        ui: UiView::default(),
                        camera: None,
                        cape: None,
                        world_watch: None,
                        world_region: None,
                        world_surface: None,
                        canvases: vec![],
                    },
                    inventories: BTreeMap::new(),
                    high_water: BTreeMap::new(),
                    last_inventory: 0,
                }
            }
        };
        let mut staged_sounds: Vec<SoundCue> = Vec::new();
        let mut staged_last = self.last_sound;
        let mut staged_ui_clock = self.ui_clock;
        for cmd in &request.commands {
            match cmd {
                SceneCommand::Cape { deformation } => {
                    if deformation
                        .as_ref()
                        .is_some_and(|d| d.identity().is_none_or(|(_, epoch)| epoch != self.epoch))
                    {
                        return Err("cape belongs to another scene epoch".into());
                    }
                    staged.view.cape = deformation.clone();
                }
                SceneCommand::Canvas { command } => {
                    let before = staged.view.canvases.clone();
                    command.apply(&mut staged.view.canvases)?;
                    if before != staged.view.canvases {
                        for canvas in &mut staged.view.canvases {
                            canvas.revision = self
                                .revision
                                .checked_add(1)
                                .ok_or("canvas revision exhausted")?;
                        }
                    }
                    staged.view.ui.widgets.retain(|widget| match widget {
                        Widget::Canvas { canvas, .. } => {
                            staged.view.canvases.iter().any(|c| c.id == *canvas)
                        }
                        _ => true,
                    });
                }
                SceneCommand::Camera { settings } => {
                    staged.view.camera = *settings;
                }
                SceneCommand::WorldWatch { watch } => {
                    staged.view.world_watch = (!watch.positions.is_empty()).then(|| watch.clone());
                    staged.view.world_region = None;
                    staged.view.world_surface = None;
                }
                SceneCommand::WorldRegion { watch } => {
                    staged.view.world_region = (watch.cells() != 0).then(|| watch.clone());
                    staged.view.world_watch = None;
                    staged.view.world_surface = None;
                }
                SceneCommand::WorldSurface { watch } => {
                    staged.view.world_surface = (watch.columns() != 0).then(|| watch.clone());
                    staged.view.world_watch = None;
                    staged.view.world_region = None;
                }
                SceneCommand::PutBlock {
                    object,
                    block,
                    position,
                    properties,
                } => {
                    catalog
                        .block_state_key(block, properties)
                        .map_err(|e| e.to_string())?;
                    let defaults =
                        match catalog.get(Kind::Block, block).map_err(|e| e.to_string())? {
                            ContentSpec::Block { components, .. } => components.clone(),
                            _ => return Err("not a block".into()),
                        };
                    let pos_milli = block_milli(*position);
                    if let Some(idx) =
                        staged.view.objects.iter().position(|o| {
                            o.object.kind == ObjectKind::Block && o.object.id == *object
                        })
                    {
                        if staged.view.objects[idx].content != *block {
                            return Err("cannot change content".into());
                        }
                        for (i, o) in staged.view.objects.iter().enumerate() {
                            if i != idx
                                && o.object.kind == ObjectKind::Block
                                && o.position_milli == pos_milli
                            {
                                return Err("cell occupied".into());
                            }
                        }
                        staged.view.objects[idx].position_milli = pos_milli;
                        staged.view.objects[idx].rotation_mdeg = [0, 0, 0];
                        staged.view.objects[idx].properties = properties.clone();
                        for be in staged.view.objects.iter_mut() {
                            if be.object.kind == ObjectKind::BlockEntity
                                && be.anchor == Some(*object)
                            {
                                be.position_milli = pos_milli;
                            }
                        }
                    } else {
                        let hw = staged
                            .high_water
                            .get(&ObjectKind::Block)
                            .copied()
                            .unwrap_or(0);
                        if *object <= hw {
                            return Err("nonmonotonic block id".into());
                        }
                        if staged.view.objects.len() >= 128 {
                            return Err("too many objects".into());
                        }
                        for o in &staged.view.objects {
                            if o.object.kind == ObjectKind::Block && o.position_milli == pos_milli {
                                return Err("cell occupied".into());
                            }
                        }
                        staged.view.objects.push(ObjectView {
                            object: ObjectRef {
                                kind: ObjectKind::Block,
                                id: *object,
                            },
                            content: block.clone(),
                            position_milli: pos_milli,
                            rotation_mdeg: [0, 0, 0],
                            properties: properties.clone(),
                            components: defaults,
                            anchor: None,
                        });
                        staged.high_water.insert(ObjectKind::Block, *object);
                    }
                }
                SceneCommand::RemoveBlock { object } => {
                    let idx = staged
                        .view
                        .objects
                        .iter()
                        .position(|o| o.object.kind == ObjectKind::Block && o.object.id == *object)
                        .ok_or_else(|| "missing block".to_string())?;
                    staged.view.objects.remove(idx);
                    staged.view.objects.retain(|o| {
                        !(o.object.kind == ObjectKind::BlockEntity && o.anchor == Some(*object))
                    });
                }
                SceneCommand::SpawnEntity {
                    object,
                    entity,
                    position_milli,
                } => {
                    let defaults = match catalog
                        .get(Kind::Entity, entity)
                        .map_err(|e| e.to_string())?
                    {
                        ContentSpec::Entity { components, .. } => components.clone(),
                        _ => return Err("not an entity".into()),
                    };
                    if staged
                        .view
                        .objects
                        .iter()
                        .any(|o| o.object.kind == ObjectKind::Entity && o.object.id == *object)
                    {
                        return Err("duplicate entity".into());
                    }
                    let hw = staged
                        .high_water
                        .get(&ObjectKind::Entity)
                        .copied()
                        .unwrap_or(0);
                    if *object <= hw {
                        return Err("nonmonotonic entity id".into());
                    }
                    if staged.view.objects.len() >= 128 {
                        return Err("too many objects".into());
                    }
                    staged.view.objects.push(ObjectView {
                        object: ObjectRef {
                            kind: ObjectKind::Entity,
                            id: *object,
                        },
                        content: entity.clone(),
                        position_milli: *position_milli,
                        rotation_mdeg: [0, 0, 0],
                        properties: BTreeMap::new(),
                        components: defaults,
                        anchor: None,
                    });
                    staged.high_water.insert(ObjectKind::Entity, *object);
                }
                SceneCommand::MoveEntity {
                    object,
                    position_milli,
                    rotation_mdeg,
                } => {
                    let target = staged
                        .view
                        .objects
                        .iter_mut()
                        .find(|o| o.object.kind == ObjectKind::Entity && o.object.id == *object)
                        .ok_or_else(|| "missing entity".to_string())?;
                    target.position_milli = *position_milli;
                    target.rotation_mdeg = *rotation_mdeg;
                }
                SceneCommand::RemoveEntity { object } => {
                    let idx = staged
                        .view
                        .objects
                        .iter()
                        .position(|o| o.object.kind == ObjectKind::Entity && o.object.id == *object)
                        .ok_or_else(|| "missing entity".to_string())?;
                    staged.view.objects.remove(idx);
                }
                SceneCommand::CreateBlockEntity {
                    object,
                    block_entity,
                    block_object,
                } => {
                    let (block_content, block_pos) = staged
                        .view
                        .objects
                        .iter()
                        .find(|o| {
                            o.object.kind == ObjectKind::Block && o.object.id == *block_object
                        })
                        .map(|o| (o.content.clone(), o.position_milli))
                        .ok_or_else(|| "missing anchor block".to_string())?;
                    let (allowed, defaults) = match catalog
                        .get(Kind::BlockEntity, block_entity)
                        .map_err(|e| e.to_string())?
                    {
                        ContentSpec::BlockEntity {
                            blocks, components, ..
                        } => (blocks.clone(), components.clone()),
                        _ => return Err("not a block entity".into()),
                    };
                    if !allowed.contains(&block_content) {
                        return Err("block entity mismatch".into());
                    }
                    if staged
                        .view
                        .objects
                        .iter()
                        .any(|o| o.object.kind == ObjectKind::BlockEntity && o.object.id == *object)
                    {
                        return Err("duplicate block entity".into());
                    }
                    let hw = staged
                        .high_water
                        .get(&ObjectKind::BlockEntity)
                        .copied()
                        .unwrap_or(0);
                    if *object <= hw {
                        return Err("nonmonotonic block entity id".into());
                    }
                    if staged.view.objects.iter().any(|o| {
                        o.object.kind == ObjectKind::BlockEntity && o.anchor == Some(*block_object)
                    }) {
                        return Err("anchor occupied".into());
                    }
                    if staged.view.objects.len() >= 128 {
                        return Err("too many objects".into());
                    }
                    staged.view.objects.push(ObjectView {
                        object: ObjectRef {
                            kind: ObjectKind::BlockEntity,
                            id: *object,
                        },
                        content: block_entity.clone(),
                        position_milli: block_pos,
                        rotation_mdeg: [0, 0, 0],
                        properties: BTreeMap::new(),
                        components: defaults,
                        anchor: Some(*block_object),
                    });
                    staged.high_water.insert(ObjectKind::BlockEntity, *object);
                }
                SceneCommand::RemoveBlockEntity { object } => {
                    let idx = staged
                        .view
                        .objects
                        .iter()
                        .position(|o| {
                            o.object.kind == ObjectKind::BlockEntity && o.object.id == *object
                        })
                        .ok_or_else(|| "missing block entity".to_string())?;
                    staged.view.objects.remove(idx);
                }
                SceneCommand::SetComponents { target, components } => {
                    target.validate_local()?;
                    catalog
                        .validate_components(components)
                        .map_err(|e| e.to_string())?;
                    let obj = staged
                        .view
                        .objects
                        .iter_mut()
                        .find(|o| o.object.kind == target.kind && o.object.id == target.id)
                        .ok_or_else(|| "missing target".to_string())?;
                    obj.components = components.clone();
                }
                SceneCommand::CreateInventory { inventory, slots } => {
                    if staged.inventories.contains_key(inventory) {
                        return Err("duplicate inventory".into());
                    }
                    if *inventory <= staged.last_inventory {
                        return Err("nonmonotonic inventory".into());
                    }
                    if staged.inventories.len() >= 8 {
                        return Err("too many inventories".into());
                    }
                    let inv = Inventory::new(*slots as usize).map_err(|e| e.to_string())?;
                    staged.inventories.insert(*inventory, inv);
                    staged.last_inventory = *inventory;
                }
                SceneCommand::SetSlot {
                    inventory,
                    slot,
                    stack,
                } => {
                    let inv = staged
                        .inventories
                        .get_mut(inventory)
                        .ok_or_else(|| "missing inventory".to_string())?;
                    inv.set(catalog, *slot as usize, stack.clone())
                        .map_err(|e| e.to_string())?;
                }
                SceneCommand::Transfer {
                    inventory,
                    from,
                    to,
                    count,
                } => {
                    let inv = staged
                        .inventories
                        .get_mut(inventory)
                        .ok_or_else(|| "missing inventory".to_string())?;
                    inv.transfer(catalog, *from as usize, *to as usize, *count)
                        .map_err(|e| e.to_string())?;
                }
                SceneCommand::Craft {
                    inventory,
                    recipe,
                    components,
                } => {
                    let inv = staged
                        .inventories
                        .get_mut(inventory)
                        .ok_or_else(|| "missing inventory".to_string())?;
                    crate::data::craft(catalog, recipe, inv, components)
                        .map_err(|e| e.to_string())?;
                }
                SceneCommand::Loot {
                    inventory,
                    table,
                    roll,
                    components,
                } => {
                    let inv = staged
                        .inventories
                        .get_mut(inventory)
                        .ok_or_else(|| "missing inventory".to_string())?;
                    let picked = crate::data::loot(catalog, table, *roll, components)
                        .map_err(|e| e.to_string())?;
                    if let Some(stack) = picked {
                        inv.insert(catalog, &stack).map_err(|e| e.to_string())?;
                    }
                }
                SceneCommand::Ui { view } => {
                    view.validate_local()?;
                    for w in &view.widgets {
                        if let Widget::Slot {
                            inventory, slot, ..
                        } = w
                        {
                            let inv = staged
                                .inventories
                                .get(inventory)
                                .ok_or_else(|| "missing inventory for slot".to_string())?;
                            if (*slot as usize) >= inv.slots().len() {
                                return Err("slot out of bounds".into());
                            }
                        }
                    }
                    if view.screen {
                        for (other_owner, other) in &self.actors {
                            if other_owner != owner && other.view.ui.screen {
                                return Err("modal conflict".into());
                            }
                        }
                    }
                    staged.ui_revisions =
                        staged
                            .ui_revisions
                            .updated(&staged.view.ui, view, &mut staged_ui_clock)?;
                    staged.view.ui = view.clone();
                }
                SceneCommand::CloseUi => {
                    staged.ui_revisions = staged.ui_revisions.updated(
                        &staged.view.ui,
                        &UiView::default(),
                        &mut staged_ui_clock,
                    )?;
                    staged.view.ui = UiView::default();
                }
                SceneCommand::PlaySound {
                    sound,
                    position_milli,
                    volume_milli,
                    pitch_milli,
                } => {
                    let seq = staged_last
                        .checked_add(1)
                        .ok_or_else(|| "sound sequence overflow".to_string())?;
                    staged_last = seq;
                    staged_sounds.push(SoundCue {
                        sequence: seq,
                        owner: owner.to_owned(),
                        sound: sound.clone(),
                        position_milli: *position_milli,
                        volume_milli: *volume_milli,
                        pitch_milli: *pitch_milli,
                    });
                }
            }
        }
        let final_view = build_actor_view(&staged);
        final_view.validate_local()?;
        let bytes = serde_json::to_vec(&final_view).map_err(|e| e.to_string())?;
        let mut metadata = final_view.clone();
        for canvas in &mut metadata.canvases {
            canvas.rgba_base64 = None;
        }
        if bytes.len() > 65536
            || serde_json::to_vec(&metadata)
                .map_err(|e| e.to_string())?
                .len()
                > 32768
        {
            return Err("actor too large".into());
        }
        if final_view.ui.screen {
            for (other_owner, other) in &self.actors {
                if other_owner != owner && other.view.ui.screen {
                    return Err("modal conflict".into());
                }
            }
        }
        let revision = self.revision.checked_add(1).ok_or("revision overflow")?;
        staged.view.inventories.clear();
        self.actors.insert(owner.to_owned(), staged);
        for cue in staged_sounds {
            self.sounds.push(cue);
        }
        if self.sounds.len() > 64 {
            let excess = self.sounds.len() - 64;
            self.sounds.drain(0..excess);
        }
        self.last_sound = staged_last;
        self.ui_clock = staged_ui_clock;
        self.revision = revision;
        self.fingerprint = Some(catalog.fingerprint().to_owned());
        Ok(())
    }
}
