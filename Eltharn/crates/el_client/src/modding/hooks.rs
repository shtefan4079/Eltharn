use el_mod_runtime::{
    hooks::{HookDispatcher, HookPoint},
    session::Definition,
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap};
#[derive(Default)]
struct Hooks {
    dispatcher: Option<HookDispatcher>,
    owners: Vec<String>,
    frame: u64,
    visibility: BTreeMap<String, (bool, bool)>,
    disabled: std::collections::BTreeSet<String>,
}
thread_local! {
    static HOOKS : RefCell < Hooks > = RefCell::new(Hooks::default());
}
pub(crate) struct PreparedHooks {
    dispatcher: HookDispatcher,
    owners: Vec<String>,
}
pub(crate) fn prepare(definitions: &[Definition]) -> Result<PreparedHooks, String> {
    let dispatcher = HookDispatcher::prepare(definitions)?;
    Ok(PreparedHooks {
        dispatcher,
        owners: definitions.iter().map(|d| d.key.clone()).collect(),
    })
}
pub(crate) fn install(prepared: PreparedHooks) {
    HOOKS.with(|cell| {
        *cell.borrow_mut() = Hooks {
            dispatcher: Some(prepared.dispatcher),
            owners: prepared.owners,
            ..Hooks::default()
        };
    });
}
pub fn reconcile(report: &Value) -> Result<(), String> {
    let states: Vec<&Value> = if let Some(states) = report.get("states") {
        states
            .as_array()
            .ok_or("Malformed hook lifecycle states")?
            .iter()
            .collect()
    } else if let Some(actors) = report.get("actors").and_then(Value::as_array) {
        let keys = HOOKS.with(|cell| cell.borrow().owners.clone());
        if actors.len() != keys.len()
            || actors
                .iter()
                .zip(&keys)
                .any(|(actor, key)| actor.get("key").and_then(Value::as_str) != Some(key))
        {
            return Err("Hook browser lifecycle roster mismatch".into());
        }
        actors
            .iter()
            .map(|actor| {
                actor
                    .get("state")
                    .ok_or("Missing hook lifecycle actor state")
            })
            .collect::<Result<_, _>>()?
    } else {
        return Err("Missing hook lifecycle roster".into());
    };
    HOOKS.with(|cell| {
        let mut hooks = cell.borrow_mut();
        if states.len() != hooks.owners.len() {
            return Err("Hook lifecycle roster mismatch".into());
        }
        if states.iter().any(|state| {
            !matches!(
                state.as_str(),
                Some("Prepared" | "Ready" | "InWorld" | "Disabled" | "Terminated")
            )
        }) {
            return Err("Invalid hook lifecycle state".into());
        }
        let dead: Vec<_> = states
            .iter()
            .zip(&hooks.owners)
            .filter(|(state, owner)| {
                matches!(state.as_str(), Some("Disabled" | "Terminated"))
                    && !hooks.disabled.contains(*owner)
            })
            .map(|(_, owner)| owner.clone())
            .collect();
        if !dead.is_empty() {
            if let Some(dispatcher) = hooks.dispatcher.as_mut() {
                for owner in &dead {
                    dispatcher.disable_owner(&owner);
                }
            }
            hooks.disabled.extend(dead);
            hooks.visibility.clear();
        }
        Ok(())
    })
}
pub fn begin_frame() {
    HOOKS.with(|cell| {
        let mut hooks = cell.borrow_mut();
        hooks.frame = hooks.frame.saturating_add(1);
        let frame = hooks.frame;
        hooks.visibility.clear();
        if let Some(dispatcher) = hooks.dispatcher.as_mut() {
            dispatcher.begin_frame(frame);
        }
    });
}
pub fn reset_world() {
    HOOKS.with(|cell| {
        let mut hooks = cell.borrow_mut();
        hooks.visibility.clear();
        if let Some(dispatcher) = hooks.dispatcher.as_mut() {
            dispatcher.reset_world();
        }
    });
}
pub(crate) fn accepts_owner(owner: &str) -> bool {
    HOOKS.with(|cell| cell.borrow().owners.iter().any(|key| key == owner))
}
pub(crate) fn publish_hook_state(owner: &str, value: Value) -> Result<(), String> {
    HOOKS.with(|cell| {
        let mut hooks = cell.borrow_mut();
        if !hooks.owners.iter().any(|key| key == owner) {
            return Err("Unknown hook-state owner".into());
        }
        let Some(dispatcher) = hooks.dispatcher.as_mut() else {
            return Ok(());
        };
        if !dispatcher.accepts_snapshot(owner) {
            return Ok(());
        }
        dispatcher.set_snapshot(owner, value)
    })
}
pub fn clear() {
    HOOKS.with(|cell| *cell.borrow_mut() = Hooks::default());
}
pub fn report() -> Value {
    HOOKS.with(|cell| {
        let hooks = cell.borrow();
        match &hooks.dispatcher {
            Some(dispatcher) => dispatcher.report(),
            None => {
                json!({ "version" : 1, "frame" : hooks.frame, "configured" : false })
            }
        }
    })
}
fn invoke(point: HookPoint, input: Value, original: impl Fn(Value) -> Value) -> (Value, bool) {
    HOOKS.with(|cell| {
        let mut hooks = cell.borrow_mut();
        let Some(dispatcher) = hooks.dispatcher.as_mut() else {
            return (original(input), false);
        };
        let outcome = dispatcher.invoke_with(point, input, original);
        if !outcome.diagnostics.is_empty() {
            hooks.visibility.clear();
        }
        (outcome.context, outcome.cancelled)
    })
}
fn has(point: HookPoint) -> bool {
    HOOKS.with(|cell| {
        cell.borrow()
            .dispatcher
            .as_ref()
            .is_some_and(|d| d.has(point))
    })
}
pub(crate) fn item_tooltip_active() -> bool {
    has(HookPoint::ItemTooltip)
}
pub(crate) enum ItemTooltipOutcome {
    Native,
    Cancelled,
    Presented(el_mod_content::tooltip::Context),
}
pub(crate) fn item_tooltip(input: &el_mod_content::tooltip::Context) -> ItemTooltipOutcome {
    if input.validate().is_err() {
        return ItemTooltipOutcome::Native;
    }
    let Ok(context) = serde_json::to_value(input) else {
        return ItemTooltipOutcome::Cancelled;
    };
    if serde_json::to_vec(&context)
        .is_ok_and(|bytes| bytes.len() > el_mod_content::tooltip::MAX_BYTES)
    {
        return ItemTooltipOutcome::Native;
    }
    let (result, cancelled) = invoke(HookPoint::ItemTooltip, context, |v| v);
    if cancelled {
        return ItemTooltipOutcome::Cancelled;
    }
    match serde_json::from_value(result) {
        Ok(result) => ItemTooltipOutcome::Presented(result),
        Err(_) => ItemTooltipOutcome::Cancelled,
    }
}
pub(crate) fn item_tooltip_layout(
    context: el_mod_content::tooltip::LayoutContext,
) -> Option<el_mod_content::tooltip::Placement> {
    context.validate().ok()?;
    if !has(HookPoint::ItemTooltipLayout) {
        return None;
    }
    let input = serde_json::to_value(context).ok()?;
    let (result, cancelled) = invoke(HookPoint::ItemTooltipLayout, input, |v| v);
    if cancelled {
        return None;
    }
    let result: el_mod_content::tooltip::LayoutContext = serde_json::from_value(result).ok()?;
    result.validate().ok()?;
    result.placement
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct CameraTransform {
    pub hurt_scale_milli: u16,
    pub rotation_radians: [f32; 3],
    pub fov_scale_milli: u16,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CameraHurt {
    pub enabled: bool,
    pub angles_degrees: [f32; 3],
}
pub(crate) fn camera_hurt(angles_degrees: [f32; 3]) -> CameraHurt {
    let baseline = CameraHurt {
        enabled: true,
        angles_degrees,
    };
    if !has(HookPoint::CameraHurt) {
        return baseline;
    }
    let (context, cancelled) = invoke(
        HookPoint::CameraHurt,
        json!(
            { "version" : 1, "enabled" : true, "death_roll_degrees" : angles_degrees[0],
            "hurt_roll_degrees" : angles_degrees[1], "hurt_yaw_degrees" :
            angles_degrees[2] }
        ),
        |value| value,
    );
    CameraHurt {
        enabled: !cancelled && context["enabled"].as_bool().unwrap_or(true),
        angles_degrees: [
            "death_roll_degrees",
            "hurt_roll_degrees",
            "hurt_yaw_degrees",
        ]
        .map(|key| context[key].as_f64().unwrap_or(0.0) as f32),
    }
}
pub(crate) fn health_blink(blinking: bool) -> bool {
    if !has(HookPoint::HudHealthBlink) {
        return blinking;
    }
    let (context, cancelled) = invoke(
        HookPoint::HudHealthBlink,
        json!({ "version" : 1, "blinking" : blinking }),
        |context| context,
    );
    !cancelled && context["blinking"].as_bool().unwrap_or(blinking)
}
pub(crate) fn camera_transform(committed_hurt_milli: u16) -> CameraTransform {
    if !has(HookPoint::CameraTransform) {
        return CameraTransform {
            hurt_scale_milli: committed_hurt_milli,
            rotation_radians: [0.0; 3],
            fov_scale_milli: 1000,
        };
    }
    let (context, cancelled) = invoke(
        HookPoint::CameraTransform,
        json!(
            { "version" : 1, "hurt_scale_milli" : 1000, "yaw_offset_radians" : 0.0,
            "pitch_offset_radians" : 0.0, "roll_offset_radians" : 0.0, "fov_scale_milli"
            : 1000 }
        ),
        |mut context| {
            let input = context["hurt_scale_milli"].as_u64().unwrap_or(1000);
            context["hurt_scale_milli"] = json!(input * u64::from(committed_hurt_milli) / 1000);
            context
        },
    );
    if cancelled {
        return CameraTransform {
            hurt_scale_milli: 0,
            rotation_radians: [0.0; 3],
            fov_scale_milli: 1000,
        };
    }
    CameraTransform {
        hurt_scale_milli: context["hurt_scale_milli"].as_u64().unwrap_or(1000) as u16,
        rotation_radians: [
            "yaw_offset_radians",
            "pitch_offset_radians",
            "roll_offset_radians",
        ]
        .map(|key| context[key].as_f64().unwrap_or(0.0) as f32),
        fov_scale_milli: context["fov_scale_milli"].as_u64().unwrap_or(1000) as u16,
    }
}
pub(crate) fn cape_rows(
    rows: Option<crate::render::model::player::cape_deform::InterpolatedCapeRows>,
) -> (
    bool,
    Option<crate::render::model::player::cape_deform::InterpolatedCapeRows>,
) {
    use el_mod_content::scene_types::avatar::CAPE_ROWS;
    if !has(HookPoint::CapeGeometry) {
        return (true, rows);
    }
    let source = rows.unwrap_or([[0.0; 3]; CAPE_ROWS]);
    let input_rows = source.map(|row| row.map(|v| v.round() as i32));
    let input = json!({ "version" : 1, "offsets_pixel_milli" : input_rows });
    let (context, cancelled) = invoke(HookPoint::CapeGeometry, input.clone(), |value| value);
    if cancelled {
        return (false, None);
    }
    if context == input {
        return (true, rows);
    }
    let output: [[i32; 3]; CAPE_ROWS] =
        serde_json::from_value(context["offsets_pixel_milli"].clone()).unwrap_or(input_rows);
    (true, Some(output.map(|row| row.map(|v| v as f32))))
}
fn visibility(point: HookPoint, name: &str, eligible: bool) -> bool {
    if !has(point) {
        return eligible;
    }
    if let Some((original, value)) = HOOKS.with(|cell| cell.borrow().visibility.get(name).copied())
        && original == eligible
    {
        return value;
    }
    let (context, cancelled) = invoke(
        point,
        json!({ "version" : 1, "visible" : eligible }),
        |mut value| {
            value["visible"] = json!(eligible && value["visible"].as_bool().unwrap_or(eligible));
            value
        },
    );
    let visible = eligible && !cancelled && context["visible"].as_bool().unwrap_or(eligible);
    HOOKS.with(|cell| {
        cell.borrow_mut()
            .visibility
            .insert(name.into(), (eligible, visible));
    });
    visible
}
pub(crate) fn hud_visible(eligible: bool) -> bool {
    visibility(HookPoint::HudVisibility, "hud", eligible)
}
pub(crate) fn hand_visible(eligible: bool) -> bool {
    visibility(HookPoint::HandVisibility, "hand", eligible)
}
pub(crate) fn world_ui_visible() -> bool {
    visibility(HookPoint::WorldUiVisibility, "world_ui", true)
}
pub(crate) fn menu_ui_visible() -> bool {
    visibility(HookPoint::MenuUiVisibility, "menu_ui", true)
}
pub(crate) fn input_action(action: &str, pressed: bool) -> Option<bool> {
    if !has(HookPoint::InputAction) {
        return Some(pressed);
    }
    let (context, cancelled) = invoke(
        HookPoint::InputAction,
        json!({ "version" : 1, "action" : action, "pressed" : pressed }),
        |value| value,
    );
    (!cancelled && context["action"].as_str() == Some(action))
        .then(|| context["pressed"].as_bool().unwrap_or(pressed))
}
