use super::resources::ModelAssets;
use crate::resources::manager::ClientResourceManager;
use el_extension_manager::content_assets::OwnedAssets;
use el_mod_content::{
    catalog::{Catalog, OwnedSpec},
    scene_types::{ChannelInput, InputEvent, InputFrame, SceneView},
};
use el_mod_network::wire::ChannelMessage;
use el_mod_runtime::content::{MAX_INPUT_BYTES, MAX_INPUT_OWNERS, MAX_OWNER_LEN};
use el_mod_runtime::lifecycle::Phase;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
const MAX_SAFE: u64 = 9007199254740991;
const MAX_PENDING: usize = 64;
const MAX_PENDING_CHANNELS: usize = 64;
const VP_MAX: u16 = 8192;
const CAM_ABS: f64 = 32_000_000.0;
#[derive(Default)]
struct ClientContent {
    catalog: Option<Arc<Catalog>>,
    local_catalog: Option<Arc<Catalog>>,
    scene: SceneView,
    local_scene: SceneView,
    local_ui_revisions:
        Option<BTreeMap<String, el_mod_content::scene_types::ui_input::UiTargetRevisions>>,
    ui_capture: super::input::UiCapture,
    remote_scene: Option<SceneView>,
    sound_merge_started: bool,
    local_sound_seen: u64,
    remote_sound_seen: u64,
    sound_sequence: u64,
    merged_sounds: Vec<el_mod_content::scene::SoundCue>,
    readers: Vec<String>,
    entity_readers: BTreeSet<String>,
    remote_readers: Vec<String>,
    assets: Vec<OwnedAssets>,
    input: InputFrame,
    world_blocks: BTreeMap<String, el_mod_content::scene_types::world::WorldBlocksSnapshot>,
    world_regions: BTreeMap<String, super::world_region::RegionCache>,
    world_surfaces: BTreeMap<String, super::surface::SurfaceCache>,
    region_sample: u64,
    pending: BTreeMap<String, Vec<InputEvent>>,
    pending_channels: BTreeMap<String, Vec<ChannelInput>>,
    dropped_channels: u64,
    event_sequence: u64,
    frame_sequence: u64,
    client_tick: u64,
    cape_controller: super::avatar::CapeController,
    world: Option<u64>,
    visible: bool,
    frame_ready: bool,
}
thread_local! {
    static STATE : RefCell < ClientContent > = RefCell::new(ClientContent::default());
}
fn validate_readers(readers: &[String]) -> Result<(), String> {
    if readers.len() > MAX_INPUT_OWNERS {
        return Err("too many readers".into());
    }
    let mut seen = BTreeSet::new();
    for r in readers {
        if r.is_empty() || r.len() > MAX_OWNER_LEN || r.contains('\0') {
            return Err("bad reader".into());
        }
        if !seen.insert(r) {
            return Err("duplicate reader".into());
        }
    }
    Ok(())
}
fn rebuild_scene(st: &mut ClientContent) -> Result<(), String> {
    let mut combined = st.local_scene.clone();
    if let Some(remote) = st.remote_scene.as_ref() {
        if combined.epoch == 0 {
            combined.epoch = remote.epoch;
        }
        combined.actors.extend(remote.actors.iter().cloned());
        combined.revision = st.scene.revision.saturating_add(1);
    }
    if st.remote_scene.is_some() && !st.sound_merge_started {
        st.sound_merge_started = true;
        st.local_sound_seen = st
            .local_scene
            .sounds
            .last()
            .map(|c| c.sequence)
            .unwrap_or(0);
        st.sound_sequence = st.scene.sounds.last().map(|c| c.sequence).unwrap_or(0);
        st.merged_sounds = st.scene.sounds.clone();
    }
    if st.sound_merge_started {
        for cue in &st.local_scene.sounds {
            if cue.sequence > st.local_sound_seen {
                st.sound_sequence = st
                    .sound_sequence
                    .checked_add(1)
                    .ok_or("sound sequence overflow")?;
                let mut cue = cue.clone();
                cue.sequence = st.sound_sequence;
                st.merged_sounds.push(cue);
            }
        }
        st.local_sound_seen = st
            .local_scene
            .sounds
            .last()
            .map(|c| c.sequence)
            .unwrap_or(st.local_sound_seen);
        if let Some(remote) = &st.remote_scene {
            for cue in &remote.sounds {
                if cue.sequence > st.remote_sound_seen {
                    st.sound_sequence = st
                        .sound_sequence
                        .checked_add(1)
                        .ok_or("sound sequence overflow")?;
                    let mut cue = cue.clone();
                    cue.sequence = st.sound_sequence;
                    st.merged_sounds.push(cue);
                }
            }
            st.remote_sound_seen = remote
                .sounds
                .last()
                .map(|c| c.sequence)
                .unwrap_or(st.remote_sound_seen);
        }
        let owners: BTreeSet<_> = combined.actors.iter().map(|a| a.owner.as_str()).collect();
        st.merged_sounds
            .retain(|c| owners.contains(c.owner.as_str()));
        if st.merged_sounds.len() > 64 {
            st.merged_sounds.drain(..st.merged_sounds.len() - 64);
        }
        combined.sounds = st.merged_sounds.clone();
    }
    combined.validate_local()?;
    st.ui_capture.reconcile(
        &st.scene,
        &combined,
        st.local_ui_revisions.as_ref(),
        &mut st.pending,
    )?;
    st.scene = combined;
    Ok(())
}
fn reset_sound_merge(st: &mut ClientContent) {
    st.sound_merge_started = false;
    st.local_sound_seen = 0;
    st.remote_sound_seen = 0;
    st.sound_sequence = 0;
    st.merged_sounds.clear();
}
fn push_event(
    pending: &mut BTreeMap<String, Vec<InputEvent>>,
    seq: &mut u64,
    owner: &str,
    event: &InputEvent,
) -> Result<(), String> {
    let cand = seq.checked_add(1).ok_or("event sequence overflow")?;
    if cand == 0 || cand > MAX_SAFE {
        return Err("event sequence overflow".into());
    }
    let mut staged = event.clone();
    staged.sequence = cand;
    staged.validate_local()?;
    let vec = pending.entry(owner.to_string()).or_default();
    if staged.kind == "pointer" && staged.key.is_none() {
        if let Some(pos) = vec
            .iter()
            .rposition(|e| e.kind == "pointer" && e.key.is_none())
        {
            vec.remove(pos);
        }
    }
    if vec.len() >= MAX_PENDING {
        return Err("pending full".into());
    }
    vec.push(staged);
    *seq = cand;
    Ok(())
}
pub fn configure(report: &serde_json::Value, readers: Vec<String>) -> Result<(), String> {
    validate_readers(&readers)?;
    let content = report.get("content");
    let catalog_value = content.and_then(|c| c.get("catalog"));
    let no_content = catalog_value.is_none() || catalog_value == Some(&serde_json::Value::Null);
    if no_content {
        if !readers.is_empty() {
            return Err("content missing but readers requested".into());
        }
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.catalog = None;
            st.local_catalog = None;
            st.readers = readers;
            st.entity_readers.clear();
            st.scene = SceneView::default();
            st.local_scene = SceneView::default();
            st.local_ui_revisions = None;
            st.ui_capture = Default::default();
            st.cape_controller.clear();
            st.remote_scene = None;
            st.remote_readers.clear();
            reset_sound_merge(&mut st);
            st.pending.clear();
            st.pending_channels.clear();
            st.world_blocks.clear();
            st.world_regions.clear();
            st.world_surfaces.clear();
            st.input.player = None;
            st.dropped_channels = 0;
            st.input = InputFrame::default();
            st.event_sequence = 0;
            st.frame_sequence = 0;
            st.visible = false;
        });
        return Ok(());
    }
    let entries: Vec<OwnedSpec> =
        serde_json::from_value(catalog_value.unwrap().clone()).map_err(|e| e.to_string())?;
    let catalog = Catalog::from_entries(entries).map_err(|e| e.to_string())?;
    let fp = content
        .and_then(|c| c.get("fingerprint"))
        .and_then(|v| v.as_str())
        .ok_or("catalog fingerprint missing")?;
    if fp != catalog.fingerprint() {
        return Err("catalog fingerprint mismatch".into());
    }
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let catalog = Arc::new(catalog);
        st.catalog = Some(catalog.clone());
        st.local_catalog = Some(catalog);
        st.readers = readers;
        st.entity_readers.clear();
        st.scene = SceneView::default();
        st.local_scene = SceneView::default();
        st.local_ui_revisions = None;
        st.ui_capture = Default::default();
        st.cape_controller.clear();
        st.remote_scene = None;
        st.remote_readers.clear();
        reset_sound_merge(&mut st);
        st.pending.clear();
        st.pending_channels.clear();
        st.world_blocks.clear();
        st.world_regions.clear();
        st.world_surfaces.clear();
        st.input.player = None;
        st.dropped_channels = 0;
        st.input = InputFrame::default();
        st.event_sequence = 0;
        st.frame_sequence = 0;
        st.visible = false;
    });
    Ok(())
}
pub fn set_assets(assets: Vec<OwnedAssets>) {
    STATE.with(|s| s.borrow_mut().assets = assets);
}
pub fn set_entity_readers(readers: BTreeSet<String>) {
    STATE.with(|s| s.borrow_mut().entity_readers = readers);
}
pub fn require_image(id: &el_mod_content::ContentId) -> Result<(), String> {
    let path = format!("assets/{}/textures/{}.png", id.namespace(), id.path());
    STATE
        .with(|s| {
            s.borrow()
                .assets
                .iter()
                .any(|a| a.files.contains_key(&path))
        })
        .then_some(())
        .ok_or_else(|| format!("UI texture absent from selected mod packages: {id}"))
}
pub fn require_owned_image(actor: &str, id: &el_mod_content::ContentId) -> Result<(), String> {
    let (package, _) = actor.rsplit_once('/').ok_or("Invalid image actor")?;
    let path = format!("assets/{}/textures/{}.png", id.namespace(), id.path());
    STATE
        .with(|s| {
            s.borrow()
                .assets
                .iter()
                .any(|assets| assets.owner.as_str() == package && assets.files.contains_key(&path))
        })
        .then_some(())
        .ok_or_else(|| "Image absent from the actor's package".into())
}
pub fn require_sound(id: &el_mod_content::ContentId) -> Result<(), String> {
    let paths =
        ["ogg", "wav"].map(|ext| format!("assets/{}/sounds/{}.{ext}", id.namespace(), id.path()));
    STATE
        .with(|s| {
            s.borrow()
                .assets
                .iter()
                .any(|a| paths.iter().any(|p| a.files.contains_key(p)))
        })
        .then_some(())
        .ok_or_else(|| format!("Sound absent from selected mod packages: {id}"))
}
pub fn mount_resources(
    base: &ClientResourceManager,
) -> Result<(ClientResourceManager, Option<ModelAssets>), String> {
    STATE.with(|s| {
        let st = s.borrow();
        super::material::validate_overlay_sounds(&st.assets)?;
        let manager = super::resources::mount(base, &st.assets)?;
        let models = match &st.catalog {
            Some(c) if !c.is_empty() => Some(super::resources::load_models(&manager, c)?),
            _ => None,
        };
        Ok((manager, models))
    })
}
pub fn catalog() -> Option<Arc<Catalog>> {
    STATE.with(|s| s.borrow().catalog.clone())
}
pub fn remote_offer(entries: &[OwnedSpec], readers: Vec<String>) -> Result<bool, String> {
    validate_readers(&readers)?;
    let remote = Catalog::from_entries(entries.to_vec())?;
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        for entry in remote.entries() {
            if let Some(local) = st.local_catalog.as_ref() {
                if local.contains(entry.spec.kind(), entry.spec.id())
                    && (local.get(entry.spec.kind(), entry.spec.id())? != &entry.spec
                        || local.owner(entry.spec.kind(), entry.spec.id())? != &entry.owner)
                {
                    return Err("server content differs from selected local mod".into());
                }
            }
        }
        let mut combined = st
            .local_catalog
            .as_ref()
            .map(|catalog| catalog.export())
            .unwrap_or_default();
        combined.extend(
            remote
                .entries()
                .filter(|entry| {
                    st.local_catalog
                        .as_ref()
                        .is_none_or(|local| !local.contains(entry.spec.kind(), entry.spec.id()))
                })
                .cloned(),
        );
        let combined = Arc::new(Catalog::from_entries(combined)?);
        let changed = st
            .local_catalog
            .as_ref()
            .is_none_or(|local| local.fingerprint() != combined.fingerprint());
        let old_catalog = st.catalog.clone();
        st.catalog = Some(combined);
        st.remote_readers = readers;
        st.remote_scene = None;
        st.remote_sound_seen = 0;
        let local = st.readers.clone();
        st.pending.retain(|owner, _| local.contains(owner));
        st.pending_channels.clear();
        st.world_blocks.clear();
        st.world_regions.clear();
        st.world_surfaces.clear();
        st.input.player = None;
        if let Err(error) = rebuild_scene(&mut st) {
            st.catalog = old_catalog;
            return Err(error);
        }
        Ok(changed)
    })
}
pub fn remote_state(scene: SceneView) -> Result<(), String> {
    scene.validate_local()?;
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let catalog = st.catalog.as_ref().ok_or("local mod catalogue missing")?;
        for actor in &scene.actors {
            if actor.cape.is_some() {
                return Err("remote actors cannot deform the local cape".into());
            }
            if !actor.canvases.is_empty() {
                return Err("remote actors cannot own client canvases".into());
            }
            if !st.remote_readers.contains(&actor.owner) {
                return Err("unannounced server actor".into());
            }
            if st.local_scene.actors.iter().any(|a| a.owner == actor.owner) {
                return Err("server actor collides with local actor".into());
            }
            for object in &actor.objects {
                let kind = match object.object.kind {
                    el_mod_content::scene_types::ObjectKind::Block => {
                        el_mod_content::spec::Kind::Block
                    }
                    el_mod_content::scene_types::ObjectKind::Entity => {
                        el_mod_content::spec::Kind::Entity
                    }
                    el_mod_content::scene_types::ObjectKind::BlockEntity => {
                        el_mod_content::spec::Kind::BlockEntity
                    }
                };
                let spec = catalog.get(kind, &object.content)?;
                match object.object.kind {
                    el_mod_content::scene_types::ObjectKind::Block => {
                        catalog.block_state_key(&object.content, &object.properties)?;
                    }
                    el_mod_content::scene_types::ObjectKind::BlockEntity => {
                        let anchor = actor
                            .objects
                            .iter()
                            .find(|candidate| {
                                candidate.object.kind
                                    == el_mod_content::scene_types::ObjectKind::Block
                                    && Some(candidate.object.id) == object.anchor
                            })
                            .ok_or("server block entity anchor absent")?;
                        if !matches!(
                            spec, el_mod_content::spec::ContentSpec::BlockEntity {
                            blocks, .. } if blocks.contains(& anchor.content) && anchor
                            .position_milli == object.position_milli
                        ) {
                            return Err("server block entity anchor mismatch".into());
                        }
                    }
                    el_mod_content::scene_types::ObjectKind::Entity => {}
                }
                catalog.validate_components(&object.components)?;
            }
            for inventory in &actor.inventories {
                for stack in inventory.slots.iter().flatten() {
                    catalog.validate_stack(stack)?;
                }
            }
        }
        st.remote_scene = Some(scene);
        rebuild_scene(&mut st)?;
        st.visible = st.world.is_some();
        Ok(())
    })
}
pub fn remote_channels(messages: &[ChannelMessage]) -> Result<(usize, usize), String> {
    STATE.with(|cell| {
        let mut st = cell.borrow_mut();
        let mut pending_channels = st.pending_channels.clone();
        let mut delivered = 0;
        let mut dropped = 0;
        for message in messages {
            if !st.remote_readers.contains(&message.owner) {
                return Err("unannounced server channel sender".into());
            }
            let input = ChannelInput {
                sender: message.owner.clone(),
                channel: message.channel.parse().map_err(|_| "bad channel id")?,
                data: message.data.clone(),
            };
            input.validate_local()?;
            let package = message.owner.rsplit_once('/').ok_or("bad channel owner")?.0;
            let receivers: Vec<_> = st
                .readers
                .iter()
                .filter(|reader| reader.rsplit_once('/').is_some_and(|(id, _)| id == package))
                .cloned()
                .collect();
            if receivers.is_empty() {
                dropped += 1;
            }
            for receiver in receivers {
                let queue = pending_channels.entry(receiver).or_default();
                if queue.len() == MAX_PENDING_CHANNELS {
                    queue.remove(0);
                    dropped += 1;
                }
                queue.push(input.clone());
                delivered += 1;
            }
        }
        st.pending_channels = pending_channels;
        st.dropped_channels = st.dropped_channels.saturating_add(dropped as u64);
        Ok((delivered, dropped))
    })
}
pub fn clear_remote() {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.remote_scene = None;
        st.remote_readers.clear();
        st.catalog = st.local_catalog.clone();
        reset_sound_merge(&mut st);
        let readers = st.readers.clone();
        st.pending.retain(|owner, _| readers.contains(owner));
        st.pending_channels.clear();
        st.world_blocks.clear();
        st.world_regions.clear();
        st.world_surfaces.clear();
        st.input.player = None;
        st.input.entities = None;
        let _ = rebuild_scene(&mut st);
    })
}
pub fn observation() -> serde_json::Value {
    STATE.with(|cell| {
        let s = cell.borrow();
        let objects: usize = s.scene.actors.iter().map(|a| a.objects.len()).sum();
        let pending: usize = s.pending.values().map(Vec::len).sum();
        serde_json::json!(
            { "visible" : s.visible, "world" : s.world, "epoch" : s.scene.epoch,
            "revision" : s.scene.revision, "actors" : s.scene.actors.len(), "objects"
            : objects, "frame_ready" : s.frame_ready, "frame_sequence" : s
            .frame_sequence, "pending" : pending, "camera_milli" : s.input
            .camera_milli, "look_milli" : s.input.look_milli, "remote_actors" : s
            .remote_scene.as_ref().map(| v | v.actors.len()).unwrap_or(0),
            "remote_readers" : s.remote_readers.len(), "dropped_channels" : s
            .dropped_channels, }
        )
    })
}
pub fn with_scene<T>(f: impl FnOnce(&Catalog, &SceneView) -> T) -> Option<T> {
    STATE.with(|s| {
        let st = s.borrow();
        if !st.visible {
            return None;
        }
        let cat = st.catalog.as_ref()?;
        Some(f(cat, &st.scene))
    })
}
pub fn accept(report: &serde_json::Value, phase: Phase) -> Result<(), String> {
    if let Some(readers) = report.get("content").and_then(|c| c.get("readers")) {
        let readers: Vec<String> =
            serde_json::from_value(readers.clone()).map_err(|e| e.to_string())?;
        validate_readers(&readers)?;
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.readers.retain(|r| readers.contains(r));
            let remaining = st.readers.clone();
            st.entity_readers.retain(|r| remaining.contains(r));
            st.pending.retain(|r, _| remaining.contains(r));
        });
    }
    match phase {
        Phase::WorldEnter => {
            let has_world = STATE.with(|s| s.borrow().world.is_some());
            if !has_world {
                return Ok(());
            }
            STATE.with(|s| s.borrow_mut().visible = true);
            let scene_value = report.get("content").and_then(|c| c.get("scene"));
            if scene_value.is_none() || scene_value == Some(&serde_json::Value::Null) {
                return Ok(());
            }
            let scene: SceneView =
                serde_json::from_value(scene_value.unwrap().clone()).map_err(|e| e.to_string())?;
            scene.validate_local()?;
            let ui_revisions = super::input::report_revisions(report, &scene)?;
            STATE.with(|s| {
                let st = s.borrow();
                if let Some(c) = st.local_catalog.as_ref() {
                    if !c.is_empty() {
                        let fp = report
                            .get("content")
                            .and_then(|cc| cc.get("fingerprint"))
                            .and_then(|v| v.as_str())
                            .ok_or("scene fingerprint missing")?;
                        if fp != c.fingerprint() {
                            return Err::<(), String>("scene fingerprint mismatch".into());
                        }
                    }
                }
                Ok(())
            })?;
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.local_scene = scene;
                st.local_ui_revisions = ui_revisions;
                rebuild_scene(&mut st)
            })?;
            Ok(())
        }
        Phase::WorldExit | Phase::Shutdown => {
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.scene = SceneView::default();
                st.local_scene = SceneView::default();
                st.local_ui_revisions = None;
                st.ui_capture = Default::default();
                st.cape_controller.clear();
                st.input.avatar = None;
                st.remote_scene = None;
                st.remote_readers.clear();
                reset_sound_merge(&mut st);
                st.visible = false;
                st.pending.clear();
                st.pending_channels.clear();
                st.world_blocks.clear();
                st.world_regions.clear();
                st.world_surfaces.clear();
                st.input.player = None;
                st.input.events.clear();
            });
            Ok(())
        }
        _ => {
            let visible = STATE.with(|s| s.borrow().visible);
            if !visible {
                return Ok(());
            }
            let scene_value = report.get("content").and_then(|c| c.get("scene"));
            if scene_value.is_none() || scene_value == Some(&serde_json::Value::Null) {
                return Ok(());
            }
            let scene: SceneView =
                serde_json::from_value(scene_value.unwrap().clone()).map_err(|e| e.to_string())?;
            scene.validate_local()?;
            let ui_revisions = super::input::report_revisions(report, &scene)?;
            STATE.with(|s| {
                let st = s.borrow();
                if let Some(c) = st.local_catalog.as_ref() {
                    if !c.is_empty() {
                        let fp = report
                            .get("content")
                            .and_then(|cc| cc.get("fingerprint"))
                            .and_then(|v| v.as_str())
                            .ok_or("scene fingerprint missing")?;
                        if fp != c.fingerprint() {
                            return Err::<(), String>("scene fingerprint mismatch".into());
                        }
                    }
                }
                Ok(())
            })?;
            STATE.with(|s| {
                let mut st = s.borrow_mut();
                st.local_scene = scene;
                st.local_ui_revisions = ui_revisions;
                rebuild_scene(&mut st)
            })?;
            Ok(())
        }
    }
}
pub fn observe_world(world: Option<u64>) -> bool {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        if st.world != world {
            st.input.entities = None;
        }
        let left_world = st.world.is_some() && st.world != world;
        if st.world != world {
            let entering_with_remote = st.world.is_none()
                && world.is_some()
                && (!st.remote_readers.is_empty() || st.remote_scene.is_some());
            st.world = world;
            st.frame_ready = false;
            st.local_scene = SceneView::default();
            st.local_ui_revisions = None;
            st.ui_capture = Default::default();
            st.cape_controller.clear();
            st.input.avatar = None;
            if entering_with_remote {
                st.visible = st.remote_scene.is_some();
                let _ = rebuild_scene(&mut st);
            } else {
                st.visible = false;
                st.scene = SceneView::default();
                st.remote_scene = None;
                st.remote_readers.clear();
                st.catalog = st.local_catalog.clone();
                reset_sound_merge(&mut st);
            }
            st.pending.clear();
            st.pending_channels.clear();
            st.world_blocks.clear();
            st.world_regions.clear();
            st.world_surfaces.clear();
            st.input.player = None;
            st.input.events.clear();
        }
        left_world
    })
}
pub fn clear() {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.input.entities = None;
        st.scene = SceneView::default();
        st.local_scene = SceneView::default();
        st.local_ui_revisions = None;
        st.ui_capture = Default::default();
        st.cape_controller.clear();
        st.input.avatar = None;
        st.remote_scene = None;
        st.remote_readers.clear();
        reset_sound_merge(&mut st);
        st.visible = false;
        st.pending.clear();
        st.pending_channels.clear();
        st.world_blocks.clear();
        st.world_regions.clear();
        st.world_surfaces.clear();
        st.input.player = None;
        st.input.events.clear();
        st.world = None;
    });
}
pub fn active() -> bool {
    STATE.with(|s| s.borrow().visible)
}
pub fn modal() -> Option<String> {
    STATE.with(|s| {
        let st = s.borrow();
        if !st.visible {
            return None;
        }
        st.scene
            .actors
            .iter()
            .find(|a| a.ui.screen)
            .map(|a| a.owner.clone())
    })
}
pub fn scene() -> SceneView {
    STATE.with(|s| {
        let st = s.borrow();
        if st.visible {
            st.scene.clone()
        } else {
            SceneView::default()
        }
    })
}
pub fn camera_hurt_scale_milli() -> u16 {
    STATE.with(|s| {
        let st = s.borrow();
        if !st.visible {
            return 1000;
        }
        st.local_scene
            .actors
            .iter()
            .filter_map(|actor| actor.camera)
            .map(|camera| camera.hurt_scale_milli)
            .min()
            .unwrap_or(1000)
    })
}
pub fn frame(camera: [f64; 3], look: [f64; 3], viewport: [u16; 2]) -> Result<(), String> {
    for c in camera {
        if !c.is_finite() || c.abs() > CAM_ABS {
            return Err("bad camera".into());
        }
    }
    for l in look {
        if !l.is_finite() || l.abs() > 1.0 {
            return Err("bad look".into());
        }
    }
    for v in viewport {
        if v > VP_MAX {
            return Err("bad viewport".into());
        }
    }
    let camera_milli = [
        camera[0].mul_add(1000.0, 0.0).round() as i64,
        camera[1].mul_add(1000.0, 0.0).round() as i64,
        camera[2].mul_add(1000.0, 0.0).round() as i64,
    ];
    let look_milli = [
        look[0].mul_add(1000.0, 0.0).round() as i32,
        look[1].mul_add(1000.0, 0.0).round() as i32,
        look[2].mul_add(1000.0, 0.0).round() as i32,
    ];
    let viewport_changed = STATE.with(|s| {
        let mut st = s.borrow_mut();
        let changed = st.frame_ready && st.input.viewport != viewport;
        if changed {
            for events in st.pending.values_mut() {
                events.retain(|event| event.widget.is_none() && event.kind != "close");
            }
        }
        st.input.camera_milli = camera_milli;
        st.input.look_milli = look_milli;
        st.input.viewport = viewport;
        st.frame_ready = true;
        st.input.events.clear();
        changed
    });
    if viewport_changed {
        super::input::reset_ui_capture();
    }
    Ok(())
}
pub fn queue(owner: &str, mut event: InputEvent) -> Result<(), String> {
    let _ = event.sequence;
    event.sequence = 1;
    STATE.with(|s| {
        let st = s.borrow();
        if !st.visible {
            return Err("not visible".to_string());
        }
        if !st
            .readers
            .iter()
            .chain(st.remote_readers.iter())
            .any(|r| r == owner)
        {
            return Err("unknown reader".to_string());
        }
        Ok(())
    })?;
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        let ClientContent {
            pending,
            event_sequence,
            ..
        } = &mut *st;
        push_event(pending, event_sequence, owner, &event)
    })
}
pub fn queue_all(event: InputEvent) -> Result<(), String> {
    let (visible, readers) = STATE.with(|s| {
        s.borrow()
            .visible
            .then(|| {
                let st = s.borrow();
                st.readers
                    .iter()
                    .chain(st.remote_readers.iter())
                    .cloned()
                    .collect()
            })
            .map_or((false, Vec::new()), |r| (true, r))
    });
    if !visible {
        return Err("not visible".into());
    }
    if readers.is_empty() {
        return Ok(());
    }
    let (mut cand_pending, mut cand_seq) =
        STATE.with(|s| (s.borrow().pending.clone(), s.borrow().event_sequence));
    for r in &readers {
        push_event(&mut cand_pending, &mut cand_seq, r, &event)?;
    }
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.pending = cand_pending;
        st.event_sequence = cand_seq;
    });
    Ok(())
}
pub fn discard_ui_input() {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        for events in state.pending.values_mut() {
            events.retain(|event| event.widget.is_none() && event.kind != "close");
        }
    });
    super::input::reset_ui_capture();
}
pub fn sample_world(level: Option<&crate::world::client_level::ClientLevel>) -> Result<(), String> {
    STATE.with(|cell| {
        let mut state = cell.borrow_mut();
        let mut samples = BTreeMap::new();
        let mut regions = BTreeMap::new();
        let mut surfaces = BTreeMap::new();
        let mut next_sample = state.region_sample;
        if state.visible
            && let Some(level) = level
        {
            for actor in &state.local_scene.actors {
                if let Some(watch) = &actor.world_surface {
                    if state.readers.contains(&actor.owner) {
                        let (surface, _) = super::surface::sample(
                            level,
                            watch,
                            &actor.owner,
                            state.local_scene.epoch,
                            &mut next_sample,
                            state.world_surfaces.get(&actor.owner),
                            super::surface::SCAN_BUDGET,
                        )?;
                        surfaces.insert(actor.owner.clone(), surface);
                    }
                }
                if let Some(watch) = &actor.world_watch {
                    if !state.readers.contains(&actor.owner) {
                        continue;
                    }
                    samples.insert(actor.owner.clone(), super::world::sample(level, watch)?);
                }
                if let Some(watch) = &actor.world_region {
                    if !state.readers.contains(&actor.owner) {
                        continue;
                    }
                    regions.insert(
                        actor.owner.clone(),
                        super::world_region::sample(
                            level,
                            watch,
                            &actor.owner,
                            state.local_scene.epoch,
                            &mut next_sample,
                            state.world_regions.get(&actor.owner),
                        )?,
                    );
                }
            }
        }
        if !samples.is_empty() || !regions.is_empty() || !surfaces.is_empty() {
            state.frame_ready = true;
        }
        state.world_blocks = samples;
        state.world_regions = regions;
        state.world_surfaces = surfaces;
        state.region_sample = next_sample;
        Ok(())
    })
}
pub fn sample_player(
    player: &crate::gameplay::player::LocalPlayer,
    dimension: Option<&str>,
) -> Result<(), String> {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.input.player = match (state.visible, state.world, state.local_scene.epoch) {
            (true, Some(world), epoch) if epoch > 0 => {
                Some(super::player::sample(player, dimension, world, epoch)?)
            }
            _ => None,
        };
        if state.input.player.is_some() {
            state.frame_ready = true;
        }
        Ok(())
    })
}
pub fn sample_entities(
    entities: &std::collections::HashMap<
        i32,
        crate::gameplay::entity::remote::RemoteEntityRenderState,
    >,
    player_position: glam::DVec3,
    tick: u64,
) -> Result<(), String> {
    use el_mod_content::scene_types::entities::{EntityObservation, EntitySnapshot, MAX_ENTITIES};
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.input.entities = None;
        if state.entity_readers.is_empty()
            || !state.visible
            || state.world.is_none()
            || state.local_scene.epoch == 0
        {
            return Ok(());
        }
        let mut nearest: Vec<_> = entities.values().collect();
        nearest.sort_by(|a, b| {
            a.position_d
                .distance_squared(player_position)
                .total_cmp(&b.position_d.distance_squared(player_position))
                .then(a.id.cmp(&b.id))
        });
        nearest.truncate(MAX_ENTITIES);
        nearest.sort_by_key(|entity| entity.id);
        let snapshot = EntitySnapshot {
            version: 1,
            epoch: state.local_scene.epoch.to_string(),
            sequence: tick.saturating_add(1).to_string(),
            total_loaded: entities.len(),
            truncated: entities.len() > MAX_ENTITIES,
            entities: nearest
                .into_iter()
                .map(|entity| EntityObservation {
                    id: entity.id,
                    kind: if entity.kind.contains(':') {
                        entity.kind.clone()
                    } else {
                        format!("minecraft:{}", entity.kind)
                    },
                    position: entity.position_d.to_array(),
                    velocity: entity.velocity.as_dvec3().to_array(),
                    yaw_degrees: entity.yaw.to_degrees(),
                    pitch_degrees: entity.pitch.to_degrees(),
                    health: entity.living_health,
                    invisible: entity.shared_flag(5),
                    crouching: matches!(
                        entity.pose,
                        el_protocol_backend::BackendEntityPose::Crouching
                    ),
                    alive: !entity.dead,
                })
                .collect(),
        };
        snapshot.validate()?;
        state.input.entities = Some(snapshot);
        state.frame_ready = true;
        Ok(())
    })
}
fn observe_cape(state: &mut ClientContent) -> Result<(), String> {
    let Some(world) = state
        .world
        .filter(|_| state.visible && state.local_scene.epoch > 0)
    else {
        state.cape_controller.clear();
        return Ok(());
    };
    state.cape_controller.accept(
        world,
        state.local_scene.epoch,
        state.client_tick,
        state
            .local_scene
            .actors
            .iter()
            .filter_map(|actor| actor.cape.as_ref().map(|cape| (actor.owner.as_str(), cape))),
    )
}
pub fn sample_avatar(
    player: &crate::gameplay::player::LocalPlayer,
    client_tick: u64,
) -> Result<(), String> {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        state.client_tick = client_tick;
        state.input.avatar = match (state.visible, state.world, state.local_scene.epoch) {
            (true, Some(world), epoch) if epoch > 0 => {
                Some(super::avatar::sample(player, world, epoch)?)
            }
            _ => None,
        };
        if state.input.avatar.is_some() {
            state.frame_ready = true;
        }
        observe_cape(&mut state)
    })
}
pub fn cape_rows(
    partial_tick: f32,
) -> Result<Option<crate::render::model::player::cape_deform::InterpolatedCapeRows>, String> {
    STATE.with(|s| {
        let mut state = s.borrow_mut();
        observe_cape(&mut state)?;
        Ok(state.cape_controller.rows(partial_tick))
    })
}
pub fn take_input() -> Result<BTreeMap<String, InputFrame>, String> {
    let (
        visible,
        readers,
        base,
        pending,
        pending_channels,
        world_blocks,
        world_regions,
        world_surfaces,
    ) = STATE.with(|s| {
        let st = s.borrow();
        (
            st.visible && st.frame_ready,
            st.readers.clone(),
            st.input.clone(),
            st.pending.clone(),
            st.pending_channels.clone(),
            st.world_blocks.clone(),
            st.world_regions.clone(),
            st.world_surfaces.clone(),
        )
    });
    if !visible || readers.is_empty() {
        return Ok(BTreeMap::new());
    }
    let frame_seq = STATE.with(|s| s.borrow().frame_sequence);
    let cand = frame_seq.checked_add(1).ok_or("frame sequence overflow")?;
    if cand == 0 || cand > MAX_SAFE {
        return Err("frame sequence overflow".into());
    }
    let mut out = BTreeMap::new();
    let mut next_pending = pending;
    let mut next_channels = pending_channels;
    for r in &readers {
        let events = next_pending.remove(r).unwrap_or_default();
        let mut fr = base.clone();
        if !STATE.with(|s| s.borrow().entity_readers.contains(r)) {
            fr.entities = None;
        }
        fr.sequence = cand;
        fr.events = events;
        fr.ui_context = STATE.with(|cell| cell.borrow().ui_capture.context(r, &fr.events));
        fr.world_blocks = world_blocks.get(r).cloned();
        fr.world_region = world_regions.get(r).map(|sample| sample.snapshot.clone());
        fr.world_surface = world_surfaces.get(r).map(|sample| sample.snapshot.clone());
        if let Some(queue) = next_channels.get_mut(r) {
            if !queue.is_empty() {
                fr.channels.push(queue.remove(0));
            }
        }
        fr.validate().map_err(|e| e.to_string())?;
        let mut bytes = serde_json::to_vec(&fr).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_INPUT_BYTES
            && let Some(region) = fr.world_region.as_mut()
        {
            region.status = el_mod_content::scene_types::world_region::RegionStatus::BudgetLimit;
            region.palette.clear();
            region.indices.clear();
            bytes = serde_json::to_vec(&fr).map_err(|e| e.to_string())?;
        }
        if bytes.len() > MAX_INPUT_BYTES {
            if let Some(surface) = fr.world_surface.as_mut() {
                surface.budget_limit();
            }
            bytes = serde_json::to_vec(&fr).map_err(|e| e.to_string())?;
        }
        if bytes.len() > MAX_INPUT_BYTES {
            return Err("input frame too large".into());
        }
        out.insert(r.clone(), fr);
    }
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        st.frame_sequence = cand;
        st.pending = next_pending;
        st.pending_channels = next_channels;
    });
    Ok(out)
}
pub fn take_remote_events(limit: usize) -> Vec<(String, InputEvent)> {
    STATE.with(|s| {
        let mut st = s.borrow_mut();
        if !st.visible || st.remote_scene.is_none() {
            return Vec::new();
        }
        let mut ordered: Vec<_> =
            st.remote_readers
                .iter()
                .flat_map(|owner| {
                    st.pending.get(owner).into_iter().flat_map(move |events| {
                        events.iter().cloned().map(move |e| (owner.clone(), e))
                    })
                })
                .collect();
        ordered.sort_by_key(|(_, event)| event.sequence);
        ordered.truncate(limit.min(32));
        let used: BTreeSet<u64> = ordered.iter().map(|(_, e)| e.sequence).collect();
        let readers = st.remote_readers.clone();
        for owner in readers {
            if let Some(events) = st.pending.get_mut(&owner) {
                events.retain(|e| !used.contains(&e.sequence));
            }
        }
        ordered
    })
}
pub fn has_readers() -> bool {
    STATE.with(|s| !s.borrow().readers.is_empty() || !s.borrow().remote_readers.is_empty())
}
pub fn can_read(owner: &str) -> bool {
    STATE.with(|s| {
        let state = s.borrow();
        state.visible
            && state
                .readers
                .iter()
                .chain(state.remote_readers.iter())
                .any(|r| r == owner)
    })
}
