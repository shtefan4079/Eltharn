use super::client_content;
use el_mod_content::{
    catalog::Catalog,
    scene_types::{InputEvent, ObjectKind, ObjectRef, ObjectView, SceneView, Widget},
    spec::{ContentSpec, Kind},
};
use glam::{DMat3, DVec3};
use std::cell::RefCell;
use std::collections::BTreeSet;
#[path = "input_revisions.rs"]
mod revisions;
pub(super) use revisions::{UiCapture, report_revisions};
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::keyboard::PhysicalKey;
#[derive(Debug, Default)]
struct LocalInput {
    focus: Option<(String, u32)>,
    pointer: [i32; 2],
    physical_pointer: Option<[f64; 2]>,
    gui_scale: Option<f64>,
    selected: Option<(String, ObjectRef)>,
    pressed_keys: BTreeSet<String>,
    last_epoch: u64,
    touch: Option<u64>,
}
std::thread_local! {
    static LOCAL : RefCell < LocalInput > = RefCell::new(LocalInput::default());
}
pub fn focus() -> Option<(String, u32)> {
    LOCAL.with(|c| c.borrow().focus.clone())
}
pub fn selected() -> Option<(String, ObjectRef)> {
    let (owner, object) = LOCAL.with(|c| c.borrow().selected.clone())?;
    let exists = client_content::with_scene(|_, scene| {
        scene.actors.iter().any(|actor| {
            actor.owner == owner && actor.objects.iter().any(|view| view.object == object)
        })
    })?;
    exists.then_some((owner, object))
}
pub fn clear() {
    LOCAL.with(|c| *c.borrow_mut() = LocalInput::default());
}
pub(super) fn reset_ui_capture() {
    LOCAL.with(|cell| {
        let mut local = cell.borrow_mut();
        local.focus = None;
        local.touch = None;
    });
}
pub fn screen_context(gui_scale: f64, vanilla_screen: bool) {
    let gui_scale = if gui_scale.is_finite() && gui_scale > 0.0 {
        gui_scale
    } else {
        1.0
    };
    let scale_changed = LOCAL.with(|cell| {
        let mut local = cell.borrow_mut();
        let changed = local.gui_scale.is_some_and(|scale| scale != gui_scale);
        local.gui_scale = Some(gui_scale);
        if changed && let Some(physical) = local.physical_pointer {
            local.pointer = to_gui(physical, gui_scale);
        }
        changed
    });
    if scale_changed || vanilla_screen {
        client_content::discard_ui_input();
    }
}
fn invalidate_ui(
    previous: &SceneView,
    scene: &SceneView,
    old: &std::collections::BTreeMap<
        String,
        el_mod_content::scene_types::ui_input::UiTargetRevisions,
    >,
    new: &std::collections::BTreeMap<
        String,
        el_mod_content::scene_types::ui_input::UiTargetRevisions,
    >,
) {
    LOCAL.with(|cell| {
        let mut local = cell.borrow_mut();
        if local.focus.as_ref().is_some_and(|(owner, id)| {
            previous.epoch != scene.epoch
                || old.get(owner).zip(new.get(owner)).is_none_or(|(old, new)| {
                    old.widgets.get(id).is_none() || old.widgets.get(id) != new.widgets.get(id)
                })
        }) {
            local.focus = None;
        }
        let modal = |scene: &SceneView| {
            scene
                .actors
                .iter()
                .find(|actor| actor.ui.screen)
                .map(|actor| actor.owner.clone())
        };
        let old_modal = modal(previous);
        let new_modal = modal(scene);
        if previous.epoch != scene.epoch
            || old_modal != new_modal
            || old_modal.as_ref().is_some_and(|owner| {
                old.get(owner)
                    .zip(new.get(owner))
                    .is_none_or(|(old, new)| old.screen != new.screen)
            })
        {
            local.touch = None;
            local.focus = None;
        }
    });
}
fn mk(
    kind: &str,
    target: Option<ObjectRef>,
    widget: Option<u32>,
    key: Option<String>,
    pressed: bool,
    pointer: [i32; 2],
) -> InputEvent {
    InputEvent {
        sequence: 1,
        kind: kind.into(),
        target,
        widget,
        key,
        pressed,
        pointer,
    }
}
fn clamp16(v: f64) -> i32 {
    (v.round() as i64).clamp(-32768, 32767) as i32
}
pub fn to_gui(physical: [f64; 2], scale: f64) -> [i32; 2] {
    let s = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };
    [clamp16(physical[0] / s), clamp16(physical[1] / s)]
}
pub fn ray_box(
    origin: [f64; 3],
    direction: [f64; 3],
    min: [f64; 3],
    max: [f64; 3],
    limit: f64,
) -> Option<f64> {
    if !limit.is_finite() || limit < 0.0 {
        return None;
    }
    for v in origin
        .iter()
        .chain(direction.iter())
        .chain(min.iter())
        .chain(max.iter())
    {
        if !v.is_finite() {
            return None;
        }
    }
    for i in 0..3 {
        if min[i] > max[i] {
            return None;
        }
    }
    let mut l2 = 0.0;
    for i in 0..3 {
        l2 += direction[i] * direction[i];
    }
    if !(l2 > 0.0) || !l2.is_finite() {
        return None;
    }
    let mut t0 = 0.0;
    let mut t1 = limit;
    for i in 0..3 {
        let (o, d, mn, mx) = (origin[i], direction[i], min[i], max[i]);
        if d.abs() <= 1e-12 {
            if o < mn || o > mx {
                return None;
            }
            continue;
        }
        let inv = 1.0 / d;
        let (mut a, mut b) = ((mn - o) * inv, (mx - o) * inv);
        if a > b {
            std::mem::swap(&mut a, &mut b);
        }
        if a > t0 {
            t0 = a;
        }
        if b < t1 {
            t1 = b;
        }
        if t0 > t1 {
            return None;
        }
    }
    if t0 > limit || t1 < 0.0 {
        return None;
    }
    Some(t0)
}
fn rot_mat(r: [i32; 3]) -> DMat3 {
    let rx = (r[0] as f64 / 1000.0).to_radians();
    let ry = (r[1] as f64 / 1000.0).to_radians();
    let rz = (r[2] as f64 / 1000.0).to_radians();
    DMat3::from_rotation_z(rz) * DMat3::from_rotation_y(ry) * DMat3::from_rotation_x(rx)
}
pub fn oriented_hit(
    origin: [f64; 3],
    dir: [f64; 3],
    base: [f64; 3],
    rot: [i32; 3],
    lmin: [f64; 3],
    lmax: [f64; 3],
    limit: f64,
) -> Option<f64> {
    let inv = rot_mat(rot).transpose();
    let o = DVec3::new(origin[0], origin[1], origin[2]) - DVec3::new(base[0], base[1], base[2]);
    let ol = inv * o;
    let dl = inv * DVec3::new(dir[0], dir[1], dir[2]);
    ray_box(ol.to_array(), dl.to_array(), lmin, lmax, limit)
}
fn base_of(pos_milli: [i64; 3]) -> [f64; 3] {
    [
        pos_milli[0] as f64 / 1000.0,
        pos_milli[1] as f64 / 1000.0,
        pos_milli[2] as f64 / 1000.0,
    ]
}
fn block_hit(
    catalog: &Catalog,
    o: &ObjectView,
    origin: [f64; 3],
    dir: [f64; 3],
    limit: f64,
) -> Option<f64> {
    let spec = catalog.get(Kind::Block, &o.content).ok()?;
    let ContentSpec::Block { selection, .. } = spec else {
        return None;
    };
    if selection.is_empty() {
        return None;
    }
    let base = base_of(o.position_milli);
    let mut best: Option<f64> = None;
    for b in selection {
        let mn = [
            b.min[0] as f64 / 16.0,
            b.min[1] as f64 / 16.0,
            b.min[2] as f64 / 16.0,
        ];
        let mx = [
            b.max[0] as f64 / 16.0,
            b.max[1] as f64 / 16.0,
            b.max[2] as f64 / 16.0,
        ];
        if let Some(d) = oriented_hit(origin, dir, base, o.rotation_mdeg, mn, mx, limit) {
            if best.is_none_or(|v| d < v) {
                best = Some(d);
            }
        }
    }
    best
}
fn entity_hit(
    catalog: &Catalog,
    o: &ObjectView,
    origin: [f64; 3],
    dir: [f64; 3],
    limit: f64,
) -> Option<f64> {
    let spec = catalog.get(Kind::Entity, &o.content).ok()?;
    let ContentSpec::Entity {
        dimensions_milli, ..
    } = spec
    else {
        return None;
    };
    let w = dimensions_milli[0] as f64 / 1000.0;
    let h = dimensions_milli[1] as f64 / 1000.0;
    if !(w > 0.0) || !(h > 0.0) || !w.is_finite() || !h.is_finite() {
        return None;
    }
    oriented_hit(
        origin,
        dir,
        base_of(o.position_milli),
        o.rotation_mdeg,
        [-w / 2.0, 0.0, -w / 2.0],
        [w / 2.0, h, w / 2.0],
        limit,
    )
}
fn pick_nearest(
    catalog: &Catalog,
    scene: &SceneView,
    origin: [f64; 3],
    dir: [f64; 3],
    limit: f64,
) -> Option<(String, ObjectRef)> {
    let mut best: Option<(f64, String, ObjectRef)> = None;
    for a in &scene.actors {
        for o in &a.objects {
            let d = match o.object.kind {
                ObjectKind::Block => block_hit(catalog, o, origin, dir, limit),
                ObjectKind::Entity => entity_hit(catalog, o, origin, dir, limit),
                ObjectKind::BlockEntity => None,
            };
            if let Some(d) = d {
                if best.as_ref().is_none_or(|(v, _, _)| d < *v) {
                    best = Some((d, a.owner.clone(), o.object));
                }
            }
        }
    }
    best.map(|(_, o, r)| (o, r))
}
pub fn widget_at(widgets: &[Widget], p: [i32; 2]) -> Option<u32> {
    for w in widgets.iter().rev() {
        match w {
            Widget::Button { id, rect, .. } | Widget::Slot { id, rect, .. } => {
                if p[0] >= rect[0]
                    && p[0] < rect[0] + rect[2]
                    && p[1] >= rect[1]
                    && p[1] < rect[1] + rect[3]
                {
                    return Some(*id);
                }
            }
            _ => {}
        }
    }
    None
}
pub fn focusables(widgets: &[Widget]) -> Vec<u32> {
    let v: Vec<u32> = widgets
        .iter()
        .filter_map(|w| match w {
            Widget::Button { id, .. } | Widget::Slot { id, .. } => Some(*id),
            _ => None,
        })
        .collect();
    v
}
pub fn focus_next(ids: &[u32], cur: Option<u32>, fwd: bool) -> Option<u32> {
    if ids.is_empty() {
        return None;
    }
    match cur.and_then(|c| ids.iter().position(|&x| x == c)) {
        None => Some(if fwd { ids[0] } else { ids[ids.len() - 1] }),
        Some(i) => Some(if fwd {
            ids[(i + 1) % ids.len()]
        } else {
            ids[(i + ids.len() - 1) % ids.len()]
        }),
    }
}
pub fn mouse_name(b: &MouseButton) -> String {
    match b {
        MouseButton::Left => "MouseLeft".into(),
        MouseButton::Right => "MouseRight".into(),
        MouseButton::Middle => "MouseMiddle".into(),
        MouseButton::Back => "MouseBack".into(),
        MouseButton::Forward => "MouseForward".into(),
        MouseButton::Other(n) => format!("MouseOther{n}"),
    }
}
pub fn update_target(
    origin: [f64; 3],
    direction: [f64; 3],
    max_distance: f64,
) -> Result<(), String> {
    for v in origin.iter().chain(direction.iter()) {
        if !v.is_finite() {
            return Err("bad ray".into());
        }
    }
    if !max_distance.is_finite() || max_distance < 0.0 {
        return Err("bad distance".into());
    }
    let mut l2 = 0.0;
    for i in 0..3 {
        l2 += direction[i] * direction[i];
    }
    if !(l2 > 0.0) || !l2.is_finite() {
        return Err("bad direction".into());
    }
    let limit = max_distance.min(1024.0);
    let nearest = client_content::with_scene(|catalog, scene| {
        pick_nearest(catalog, scene, origin, direction, limit)
    })
    .flatten();
    let epoch = client_content::with_scene(|_, scene| scene.epoch).unwrap_or(0);
    LOCAL.with(|c| {
        let mut local = c.borrow_mut();
        if local.last_epoch != epoch {
            *local = LocalInput {
                last_epoch: epoch,
                ..Default::default()
            };
        }
    });
    LOCAL.with(|c| {
        let l = c.borrow();
        if l.selected == nearest {
            return Ok(());
        }
        let prev = l.selected.clone();
        let ptr = l.pointer;
        drop(l);
        if let Some((po, _)) = &prev {
            let need = nearest.as_ref().is_none_or(|(no, _)| no != po);
            if need && client_content::can_read(po) {
                client_content::queue(po, mk("select", None, None, None, false, ptr))?;
            }
        }
        if let Some((no, nr)) = &nearest {
            if client_content::can_read(no) {
                client_content::queue(no, mk("select", Some(*nr), None, None, false, ptr))?;
            }
        }
        LOCAL.with(|c| c.borrow_mut().selected = nearest);
        Ok(())
    })
}
fn modal_widgets(modal: &str) -> Vec<Widget> {
    client_content::with_scene(|_, scene| {
        scene
            .actors
            .iter()
            .find(|a| a.owner == modal)
            .map(|a| a.ui.widgets.clone())
            .unwrap_or_default()
    })
    .unwrap_or_default()
}
fn focus_valid(owner: &str, id: u32) -> bool {
    let ws = modal_widgets(owner);
    ws.iter()
        .any(|w| w.id() == id && matches!(w, Widget::Button { .. } | Widget::Slot { .. }))
}
#[allow(clippy::too_many_lines)]
pub fn handle(event: &WindowEvent, gui_scale: f64, vanilla_screen: bool) -> Result<bool, String> {
    screen_context(gui_scale, vanilla_screen);
    if vanilla_screen || !client_content::active() {
        client_content::discard_ui_input();
        clear();
        return Ok(false);
    }
    let ui_visible = super::hooks::world_ui_visible();
    if !ui_visible {
        client_content::discard_ui_input();
    }
    let gui_scale = if gui_scale.is_finite() && gui_scale > 0.0 {
        gui_scale
    } else {
        1.0
    };
    let epoch = client_content::with_scene(|_, s| s.epoch).unwrap_or(0);
    LOCAL.with(|c| {
        let mut l = c.borrow_mut();
        if l.last_epoch != epoch {
            l.focus = None;
            l.pressed_keys.clear();
            l.selected = None;
            l.touch = None;
            l.last_epoch = epoch;
        }
        if let Some((o, id)) = l.focus.clone() {
            drop(l);
            let ok = focus_valid(&o, id);
            l = c.borrow_mut();
            if !ok {
                l.focus = None;
            }
        }
        Ok::<(), String>(())
    })?;
    let modal = ui_visible.then(client_content::modal).flatten();
    match event {
        WindowEvent::CursorMoved { position, .. } => {
            let p = to_gui([position.x, position.y], gui_scale);
            let owner = modal.clone();
            LOCAL.with(|c| {
                let mut l = c.borrow_mut();
                l.pointer = p;
                l.physical_pointer = Some([position.x, position.y]);
                if let Some(o) = owner.clone() {
                    let ws = modal_widgets(&o);
                    l.focus = widget_at(&ws, p).map(|id| (o.clone(), id));
                    let ptr = l.pointer;
                    let foc = l.focus.clone();
                    drop(l);
                    let wid = foc.map(|(_, id)| id);
                    if client_content::can_read(&o) {
                        client_content::queue(&o, mk("pointer", None, wid, None, false, ptr))?;
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            })
        }
        WindowEvent::MouseInput { state, button, .. } => {
            let name = mouse_name(button);
            let pressed = *state == ElementState::Pressed;
            if let Some(o) = modal.clone() {
                LOCAL.with(|c| {
                    let mut l = c.borrow_mut();
                    let ptr = l.pointer;
                    let ws = modal_widgets(&o);
                    let hit = widget_at(&ws, ptr);
                    if pressed {
                        l.focus = hit.map(|id| (o.clone(), id));
                        drop(l);
                        if client_content::can_read(&o) {
                            if let Some(id) = hit {
                                client_content::queue(
                                    &o,
                                    mk("activate", None, Some(id), Some(name), true, ptr),
                                )?;
                            } else {
                                client_content::queue(
                                    &o,
                                    mk("pointer", None, None, Some(name), true, ptr),
                                )?;
                            }
                        }
                    } else {
                        drop(l);
                        if client_content::can_read(&o) {
                            client_content::queue(
                                &o,
                                mk("pointer", None, hit, Some(name), false, ptr),
                            )?;
                        }
                    }
                    Ok(true)
                })
            } else {
                LOCAL.with(|c| {
                    let sel = c.borrow().selected.clone();
                    let ptr = c.borrow().pointer;
                    if let Some((o, r)) = sel {
                        if client_content::can_read(&o) {
                            client_content::queue(
                                &o,
                                mk("activate", Some(r), None, Some(name), pressed, ptr),
                            )?;
                        }
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                })
            }
        }
        WindowEvent::KeyboardInput { event, .. } => {
            let PhysicalKey::Code(code) = event.physical_key else {
                return Ok(modal.is_some());
            };
            let name = format!("{code:?}");
            if name.is_empty() || name.len() > 48 || !name.bytes().all(|b| b >= 0x20 && b <= 0x7E) {
                return Ok(modal.is_some());
            }
            let pressed = event.state == ElementState::Pressed;
            if let Some(o) = modal.clone() {
                LOCAL.with(|c| {
                    let mut l = c.borrow_mut();
                    if pressed {
                        if l.pressed_keys.contains(&name) {
                            return Ok(true);
                        }
                        if l.pressed_keys.len() >= 64 {
                            return Err("too many keys".into());
                        }
                        l.pressed_keys.insert(name.clone());
                        let shift = l.pressed_keys.contains("ShiftLeft")
                            || l.pressed_keys.contains("ShiftRight");
                        let ptr = l.pointer;
                        let cur = l.focus.clone();
                        drop(l);
                        let ws = modal_widgets(&o);
                        let ids = focusables(&ws);
                        match name.as_str() {
                            "Tab" | "ArrowUp" | "ArrowDown" | "ArrowLeft" | "ArrowRight" => {
                                let fwd = if name == "Tab" {
                                    !shift
                                } else {
                                    name == "ArrowDown" || name == "ArrowRight"
                                };
                                let cur_id = cur.filter(|(fo, _)| fo == &o).map(|(_, id)| id);
                                let next = focus_next(&ids, cur_id, fwd);
                                LOCAL.with(|c2| {
                                    c2.borrow_mut().focus = next.map(|id| (o.clone(), id))
                                });
                                if client_content::can_read(&o) {
                                    client_content::queue(
                                        &o,
                                        mk("pointer", None, next, None, false, ptr),
                                    )?;
                                }
                                return Ok(true);
                            }
                            "Enter" | "Space" => {
                                if let Some((fo, id)) = cur {
                                    if fo == o && client_content::can_read(&o) {
                                        client_content::queue(
                                            &o,
                                            mk("activate", None, Some(id), Some(name), true, ptr),
                                        )?;
                                    }
                                }
                                return Ok(true);
                            }
                            "Escape" => {
                                if client_content::can_read(&o) {
                                    client_content::queue(
                                        &o,
                                        mk("close", None, None, None, false, ptr),
                                    )?;
                                }
                                return Ok(true);
                            }
                            _ => {
                                if client_content::can_read(&o) {
                                    let wid = cur.filter(|(fo, _)| fo == &o).map(|(_, id)| id);
                                    client_content::queue(
                                        &o,
                                        mk("key", None, wid, Some(name), true, ptr),
                                    )?;
                                }
                                return Ok(true);
                            }
                        }
                    }
                    l.pressed_keys.remove(&name);
                    let ptr = l.pointer;
                    let cur = l.focus.clone();
                    drop(l);
                    if name == "Escape" {
                        return Ok(true);
                    }
                    if client_content::can_read(&o) {
                        let wid = cur.filter(|(fo, _)| fo == &o).map(|(_, id)| id);
                        client_content::queue(&o, mk("key", None, wid, Some(name), false, ptr))?;
                    }
                    Ok(true)
                })
            } else {
                LOCAL.with(|c| {
                    let mut l = c.borrow_mut();
                    if pressed {
                        if l.pressed_keys.contains(&name) {
                            return Ok(false);
                        }
                        if l.pressed_keys.len() >= 64 {
                            return Err("too many keys".into());
                        }
                        l.pressed_keys.insert(name.clone());
                    } else {
                        l.pressed_keys.remove(&name);
                    }
                    let ptr = l.pointer;
                    drop(l);
                    client_content::queue_all(mk("key", None, None, Some(name), pressed, ptr))?;
                    Ok(false)
                })
            }
        }
        WindowEvent::Touch(t) => {
            let p = to_gui([t.location.x, t.location.y], gui_scale);
            let id = t.id;
            match t.phase {
                winit::event::TouchPhase::Started => LOCAL.with(|c| {
                    let mut l = c.borrow_mut();
                    if l.touch.is_some() {
                        return Ok(modal.is_some());
                    }
                    l.touch = Some(id);
                    l.pointer = p;
                    l.physical_pointer = Some([t.location.x, t.location.y]);
                    if let Some(o) = modal.clone() {
                        let ws = modal_widgets(&o);
                        let hit = widget_at(&ws, p);
                        l.focus = hit.map(|wid| (o.clone(), wid));
                        drop(l);
                        if client_content::can_read(&o) {
                            if let Some(wid) = hit {
                                client_content::queue(
                                    &o,
                                    mk("activate", None, Some(wid), Some("Touch".into()), true, p),
                                )?;
                            } else {
                                client_content::queue(
                                    &o,
                                    mk("pointer", None, None, Some("Touch".into()), true, p),
                                )?;
                            }
                        }
                        Ok(true)
                    } else {
                        let sel = l.selected.clone();
                        drop(l);
                        if let Some((o, r)) = sel {
                            if client_content::can_read(&o) {
                                client_content::queue(
                                    &o,
                                    mk("activate", Some(r), None, Some("Touch".into()), true, p),
                                )?;
                            }
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    }
                }),
                winit::event::TouchPhase::Ended | winit::event::TouchPhase::Cancelled => LOCAL
                    .with(|c| {
                        let mut l = c.borrow_mut();
                        if l.touch != Some(id) {
                            return Ok(modal.is_some());
                        }
                        l.touch = None;
                        l.pointer = p;
                        l.physical_pointer = Some([t.location.x, t.location.y]);
                        drop(l);
                        if let Some(o) = modal.clone() {
                            if client_content::can_read(&o) {
                                client_content::queue(
                                    &o,
                                    mk("pointer", None, None, Some("Touch".into()), false, p),
                                )?;
                            }
                            Ok(true)
                        } else {
                            let sel = LOCAL.with(|c2| c2.borrow().selected.clone());
                            if let Some((o, _)) = sel {
                                if client_content::can_read(&o) {
                                    client_content::queue(
                                        &o,
                                        mk("pointer", None, None, Some("Touch".into()), false, p),
                                    )?;
                                }
                                Ok(true)
                            } else {
                                Ok(false)
                            }
                        }
                    }),
                winit::event::TouchPhase::Moved => LOCAL.with(|c| {
                    let mut l = c.borrow_mut();
                    if l.touch != Some(id) {
                        return Ok(modal.is_some());
                    }
                    l.pointer = p;
                    l.physical_pointer = Some([t.location.x, t.location.y]);
                    drop(l);
                    if let Some(o) = modal.clone() {
                        if client_content::can_read(&o) {
                            client_content::queue(&o, mk("pointer", None, None, None, false, p))?;
                        }
                        Ok(true)
                    } else {
                        Ok(false)
                    }
                }),
            }
        }
        WindowEvent::Focused(f) => {
            if !f {
                LOCAL.with(|c| {
                    let mut l = c.borrow_mut();
                    if !client_content::active() {
                        l.pressed_keys.clear();
                        l.focus = None;
                        return Ok(false);
                    }
                    let keys: Vec<String> = l.pressed_keys.iter().cloned().collect();
                    let ptr = l.pointer;
                    l.pressed_keys.clear();
                    l.focus = None;
                    drop(l);
                    for k in keys {
                        client_content::queue_all(mk("key", None, None, Some(k), false, ptr))?;
                    }
                    Ok(false)
                })
            } else {
                Ok(false)
            }
        }
        _ => Ok(false),
    }
}
