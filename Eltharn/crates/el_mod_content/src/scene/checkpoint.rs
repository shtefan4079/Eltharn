use super::*;
use std::collections::BTreeSet;

const CHECKPOINT_VERSION: u32 = 1;
const MAX_CHECKPOINT_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorCheckpoint {
    pub view: ActorView,
    pub high_water: BTreeMap<ObjectKind, u32>,
    pub last_inventory: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCheckpoint {
    pub version: u32,
    pub actors: Vec<ActorCheckpoint>,
}

fn bounded(checkpoint: &SceneCheckpoint) -> Result<(), String> {
    if checkpoint.version != CHECKPOINT_VERSION || checkpoint.actors.len() > 16 {
        return Err("unsupported scene checkpoint".into());
    }
    if serde_json::to_vec(checkpoint)
        .map_err(|_| "checkpoint encoding failed")?
        .len()
        > MAX_CHECKPOINT_BYTES
    {
        return Err("scene checkpoint too large".into());
    }
    Ok(())
}

fn normalized(mut view: ActorView) -> ActorView {
    view.objects.sort_by_key(|o| (o.object.kind, o.object.id));
    view.inventories.sort_by_key(|i| i.id);
    view.ui.screen = false;
    view.camera = None;
    view.cape = None;
    view.world_watch = None;
    view.world_region = None;
    view.world_surface = None;
    view.canvases.clear();
    view.ui
        .widgets
        .retain(|widget| !matches!(widget, Widget::Canvas { .. }));
    view
}

impl SceneState {
    pub fn checkpoint(&self) -> Result<SceneCheckpoint, String> {
        if !self.active {
            return Err("scene not active".into());
        }
        let checkpoint = SceneCheckpoint {
            version: CHECKPOINT_VERSION,
            actors: self
                .actors
                .values()
                .map(|state| ActorCheckpoint {
                    view: {
                        let mut view = build_actor_view(state);
                        view.camera = None;
                        view.cape = None;
                        view.world_watch = None;
                        view.world_region = None;
                        view.world_surface = None;
                        view.canvases.clear();
                        view.ui
                            .widgets
                            .retain(|widget| !matches!(widget, Widget::Canvas { .. }));
                        view
                    },
                    high_water: state.high_water.clone(),
                    last_inventory: state.last_inventory,
                })
                .collect(),
        };
        bounded(&checkpoint)?;
        Ok(checkpoint)
    }

    pub fn restore(
        catalog: &Catalog,
        checkpoint: &SceneCheckpoint,
        epoch: u64,
    ) -> Result<Self, String> {
        bounded(checkpoint)?;
        if epoch == 0 {
            return Err("bad restore epoch".into());
        }
        let mut scene = Self::new();
        scene.enter(epoch)?;
        let mut owners = BTreeSet::new();
        for saved in &checkpoint.actors {
            let owner = &saved.view.owner;
            saved.view.validate_local()?;
            owner_ok(owner)?;
            let (package, slot) = owner.rsplit_once('/').ok_or("bad checkpoint owner")?;
            if slot.is_empty() || slot.contains('/') {
                return Err("bad checkpoint slot".into());
            }
            package
                .parse::<el_extension_api::ExtensionId>()
                .map_err(|_| "bad checkpoint package")?;
            if !owners.insert(owner.clone()) {
                return Err("duplicate checkpoint actor".into());
            }
            let mut objects = saved.view.objects.clone();
            objects.sort_by_key(|o| (o.object.kind, o.object.id));
            let apply_one =
                |command: SceneCommand, target: &mut SceneState| -> Result<(), String> {
                    target.apply(
                        catalog,
                        owner,
                        &SceneRequest {
                            epoch,
                            commands: vec![command],
                        },
                    )
                };
            for object in &objects {
                let command = match object.object.kind {
                    ObjectKind::Block => {
                        let p = object.position_milli;
                        if p.iter().any(|n| {
                            n % 1000 != 0
                                || *n / 1000 < i32::MIN as i64
                                || *n / 1000 > i32::MAX as i64
                        }) {
                            return Err("invalid saved block position".into());
                        }
                        SceneCommand::PutBlock {
                            object: object.object.id,
                            block: object.content.clone(),
                            position: [
                                (p[0] / 1000) as i32,
                                (p[1] / 1000) as i32,
                                (p[2] / 1000) as i32,
                            ],
                            properties: object.properties.clone(),
                        }
                    }
                    ObjectKind::Entity => SceneCommand::SpawnEntity {
                        object: object.object.id,
                        entity: object.content.clone(),
                        position_milli: object.position_milli,
                    },
                    ObjectKind::BlockEntity => SceneCommand::CreateBlockEntity {
                        object: object.object.id,
                        block_entity: object.content.clone(),
                        block_object: object.anchor.ok_or("missing anchor")?,
                    },
                };
                apply_one(command, &mut scene)?;
            }
            for object in &objects {
                apply_one(
                    SceneCommand::SetComponents {
                        target: object.object,
                        components: object.components.clone(),
                    },
                    &mut scene,
                )?;
                if object.object.kind == ObjectKind::Entity && object.rotation_mdeg != [0, 0, 0] {
                    apply_one(
                        SceneCommand::MoveEntity {
                            object: object.object.id,
                            position_milli: object.position_milli,
                            rotation_mdeg: object.rotation_mdeg,
                        },
                        &mut scene,
                    )?;
                }
            }
            let mut inventories = saved.view.inventories.clone();
            inventories.sort_by_key(|i| i.id);
            for inv in &inventories {
                apply_one(
                    SceneCommand::CreateInventory {
                        inventory: inv.id,
                        slots: inv.slots.len() as u16,
                    },
                    &mut scene,
                )?;
                for (index, stack) in inv.slots.iter().enumerate() {
                    if let Some(stack) = stack {
                        apply_one(
                            SceneCommand::SetSlot {
                                inventory: inv.id,
                                slot: index as u16,
                                stack: Some(stack.clone()),
                            },
                            &mut scene,
                        )?;
                    }
                }
            }
            let mut ui = saved.view.ui.clone();
            ui.screen = false;
            apply_one(SceneCommand::Ui { view: ui }, &mut scene)?;
            let rebuilt = normalized(scene.snapshot(owner).actor);
            if rebuilt != normalized(saved.view.clone()) {
                return Err("scene checkpoint differs from checked reconstruction".into());
            }
            let state = scene
                .actors
                .get_mut(owner)
                .ok_or("restored actor missing")?;
            for (kind, used) in &state.high_water {
                if saved.high_water.get(kind).copied().unwrap_or_default() < *used {
                    return Err("checkpoint object high-water below live ID".into());
                }
            }
            if saved.last_inventory < state.last_inventory {
                return Err("checkpoint inventory high-water below live ID".into());
            }
            state.high_water = saved.high_water.clone();
            state.last_inventory = saved.last_inventory;
        }
        Ok(scene)
    }
}
