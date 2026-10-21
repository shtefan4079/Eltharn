use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[path = "keybindings_keys.rs"]
mod keys;
pub use keys::{KEYBOARD_KEYS, KEYBOARD_PHYSICAL_KEYS};

pub const CAPABILITY: &str = el_extension_api::services::CLIENT_KEYBINDINGS_CAPABILITY;
pub const REGISTER_OPCODE: i32 = el_extension_api::services::CLIENT_KEYBINDINGS_REGISTER_OPCODE;
pub const READ_OPCODE: i32 = el_extension_api::services::CLIENT_KEYBINDING_READ_OPCODE;
pub const MAX_BINDINGS: usize = 32;
pub const MAX_SEQUENCE: u64 = (1u64 << 53) - 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub id: String,
    pub category: String,
    pub label: String,
    pub default_binding: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_binding: Option<String>,
    #[serde(default)]
    pub order: i32,
}
impl Binding {
    pub fn validate(&self) -> Result<(), String> {
        validate_id(&self.id)?;
        for text in [&self.category, &self.label] {
            if text.is_empty() || text.len() > 128 || text.chars().any(char::is_control) {
                return Err("Invalid keybinding display text".into());
            }
        }
        validate_binding(&self.default_binding)?;
        if let Some(initial) = &self.initial_binding {
            validate_binding(initial)?;
        }
        Ok(())
    }
}
pub fn validate_id(id: &str) -> Result<(), String> {
    if id.is_empty()
        || id.len() > 64
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-/".contains(&c))
    {
        return Err("Invalid keybinding id".into());
    }
    Ok(())
}
pub fn validate_binding(binding: &str) -> Result<(), String> {
    if let Some(key) = binding.strip_prefix("key.keyboard.") {
        if key == "unknown" || KEYBOARD_KEYS.contains(&key) {
            return Ok(());
        }
    }
    if let Some(mouse) = binding.strip_prefix("key.mouse.") {
        if matches!(mouse, "left" | "right" | "middle")
            || mouse
                .parse::<u32>()
                .is_ok_and(|n| n > 0 && n <= 65536 && n.to_string() == mouse)
        {
            return Ok(());
        }
    }
    Err("Unsupported keybinding".into())
}
pub fn binding_from_physical(physical: &str) -> Option<String> {
    if let Some((_, token)) = KEYBOARD_PHYSICAL_KEYS
        .iter()
        .find(|(name, _)| *name == physical)
    {
        return Some(format!("key.keyboard.{token}"));
    }
    let mouse = match physical {
        "MouseLeft" => 0,
        "MouseRight" => 1,
        "MouseMiddle" => 2,
        "MouseBack" => 3,
        "MouseForward" => 4,
        _ => {
            let suffix = physical.strip_prefix("MouseOther")?;
            let value = suffix.parse::<u16>().ok()?;
            if value.to_string() != suffix {
                return None;
            }
            value
        }
    };
    Some(format!("key.mouse.{}", u32::from(mouse) + 1))
}
pub fn validate_bindings(bindings: &[Binding]) -> Result<(), String> {
    if bindings.len() > MAX_BINDINGS {
        return Err("Keybinding count limit".into());
    }
    let mut ids = BTreeSet::new();
    for binding in bindings {
        binding.validate()?;
        if !ids.insert(&binding.id) {
            return Err("Duplicate keybinding id".into());
        }
    }
    Ok(())
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputState {
    pub id: String,
    pub binding: String,
    pub down: bool,
    pub clicks: u32,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputFrame {
    pub sequence: u64,
    pub states: Vec<InputState>,
}
impl InputFrame {
    pub fn validate(&self) -> Result<(), String> {
        if self.sequence == 0 || self.sequence > MAX_SEQUENCE || self.states.len() > MAX_BINDINGS {
            return Err("Invalid keybinding frame".into());
        }
        let mut ids = BTreeSet::new();
        for state in &self.states {
            validate_id(&state.id)?;
            validate_binding(&state.binding)?;
            if state.clicks > 65535 || !ids.insert(&state.id) {
                return Err("Invalid keybinding input state".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub bindings: BTreeMap<String, Vec<Binding>>,
}
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub states: Vec<InputState>,
}
impl Snapshot {
    pub fn validate(&self) -> Result<(), String> {
        InputFrame {
            sequence: 1,
            states: self.states.clone(),
        }
        .validate()
    }
}
#[derive(Clone, Default)]
pub(crate) struct State {
    bindings: BTreeMap<String, Vec<Binding>>,
    inputs: BTreeMap<String, InputFrame>,
}
impl State {
    pub fn register(&mut self, owner: &str, bindings: &[Binding]) -> Result<(), String> {
        let mut combined = self.bindings.get(owner).cloned().unwrap_or_default();
        combined.extend_from_slice(bindings);
        validate_bindings(&combined)?;
        self.bindings.insert(owner.into(), combined);
        Ok(())
    }
    pub fn snapshot(&self, owner: &str) -> Snapshot {
        let inputs = self.inputs.get(owner);
        Snapshot {
            states: self
                .bindings
                .get(owner)
                .into_iter()
                .flatten()
                .map(|binding| {
                    inputs
                        .and_then(|frame| frame.states.iter().find(|s| s.id == binding.id))
                        .cloned()
                        .unwrap_or_else(|| InputState {
                            id: binding.id.clone(),
                            binding: binding
                                .initial_binding
                                .as_ref()
                                .unwrap_or(&binding.default_binding)
                                .clone(),
                            down: false,
                            clicks: 0,
                        })
                })
                .collect(),
        }
    }
    pub fn report(&self) -> Report {
        Report {
            bindings: self.bindings.clone(),
        }
    }
    pub fn set_input(&mut self, frames: BTreeMap<String, InputFrame>) -> Result<(), String> {
        if frames.len() > 16 {
            return Err("Keybinding input owner limit".into());
        }
        for (owner, frame) in &frames {
            frame.validate()?;
            if self
                .inputs
                .get(owner)
                .is_some_and(|old| frame.sequence <= old.sequence)
            {
                return Err("Stale keybinding input".into());
            }
            let bindings = self
                .bindings
                .get(owner)
                .ok_or("Unregistered keybinding owner")?;
            let expected: BTreeSet<_> = bindings.iter().map(|b| b.id.as_str()).collect();
            let actual: BTreeSet<_> = frame.states.iter().map(|s| s.id.as_str()).collect();
            if expected != actual {
                return Err("Keybinding input does not match registration".into());
            }
        }
        self.inputs.extend(frames);
        Ok(())
    }
    pub fn finish_tick(&mut self) {
        for frame in self.inputs.values_mut() {
            for state in &mut frame.states {
                state.clicks = 0;
            }
        }
    }
    pub fn clear_pressed(&mut self) {
        for frame in self.inputs.values_mut() {
            for state in &mut frame.states {
                state.down = false;
                state.clicks = 0;
            }
        }
    }
    pub fn disable(&mut self, owner: &str) {
        self.bindings.remove(owner);
        self.inputs.remove(owner);
    }
}
