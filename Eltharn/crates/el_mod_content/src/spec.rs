use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{ContentId, Definition, StateProperty, StateSchema};

pub const MAX_COMPONENTS_PER_SPEC: usize = 32;
pub const MAX_FIELDS_PER_COMPONENT_VALUE: usize = 32;
pub const MAX_COMPONENT_VALUE_BYTES: usize = 4096;
pub const MAX_STATE_MODELS: usize = 4096;
pub const MAX_BOXES: usize = 32;
pub const MAX_LIGHT: u8 = 15;
pub const MAX_HARDNESS_MILLI: u32 = 1_000_000_000;
pub const MAX_STACK: u16 = 1024;
pub const MAX_ENTITY_DIM_MILLI: u16 = 16000;
pub const MAX_BLOCKENTITY_BLOCKS: usize = 64;
pub const MAX_COMPONENT_FIELDS: usize = 32;
pub const MAX_TAG_MEMBERS: usize = 256;
pub const MAX_TAG_INCLUDES: usize = 64;
pub const MAX_RECIPE_INPUTS: usize = 64;
pub const MAX_LOOT_ENTRIES: usize = 64;
pub const MAX_CONDITION_REFS: usize = 64;
pub const MAX_FIELD_NAME_LEN: usize = 64;
pub const MAX_TEXT_BYTES: u16 = 4096;

fn is_field_name(s: &str) -> bool {
    if s.is_empty() || s.len() > MAX_FIELD_NAME_LEN {
        return false;
    }
    s.bytes()
        .all(|b| matches!(b, b'a'..=b'z' | b'0'..=b'9' | b'_'))
}

fn validate_field_name(s: &str) -> Result<(), String> {
    if !is_field_name(s) {
        return Err("invalid field name".to_string());
    }
    Ok(())
}

fn validate_json_value(v: &serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_COMPONENT_VALUE_BYTES {
        return Err(format!("value too large: {}", bytes.len()));
    }
    Definition::parse(&bytes)
        .map(|_| ())
        .map_err(|e| format!("invalid value: {e}"))
}

fn validate_components(c: &Components) -> Result<(), String> {
    if c.len() > MAX_COMPONENTS_PER_SPEC {
        return Err(format!("too many components: {}", c.len()));
    }
    for fields in c.values() {
        if fields.len() > MAX_FIELDS_PER_COMPONENT_VALUE {
            return Err("too many fields in component value".to_string());
        }
        for (k, v) in fields {
            validate_field_name(k)?;
            validate_json_value(v)?;
        }
    }
    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Block,
    Item,
    Entity,
    BlockEntity,
    Component,
    Tag,
    Recipe,
    Loot,
    Condition,
}

impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Block => "block",
            Kind::Item => "item",
            Kind::Entity => "entity",
            Kind::BlockEntity => "block_entity",
            Kind::Component => "component",
            Kind::Tag => "tag",
            Kind::Recipe => "recipe",
            Kind::Loot => "loot",
            Kind::Condition => "condition",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Model,
    Texture,
    Sound,
    Language,
    Material,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetRef {
    pub kind: AssetKind,
    pub id: ContentId,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Box16 {
    pub min: [u8; 3],
    pub max: [u8; 3],
}

impl Box16 {
    pub fn validate(&self) -> Result<(), String> {
        for i in 0..3 {
            if self.max[i] > 16 {
                return Err("box bound > 16".to_string());
            }
            if self.min[i] >= self.max[i] {
                return Err("box requires min < max".to_string());
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StateSpec {
    pub properties: BTreeMap<String, Vec<String>>,
    pub default: BTreeMap<String, String>,
}

impl StateSpec {
    pub fn schema(&self) -> Result<StateSchema, String> {
        if self.properties.len() > crate::MAX_STATE_PROPERTIES
            || self.default.len() != self.properties.len()
        {
            return Err("invalid property/default count".into());
        }
        let mut props = Vec::with_capacity(self.properties.len());
        for (k, v) in &self.properties {
            props.push(StateProperty::new(k.clone(), v.clone()).map_err(|e| e.to_string())?);
        }
        let defaults: Vec<(String, String)> = self
            .default
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        StateSchema::new(props, defaults)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum FieldType {
    Bool,
    Integer { min: i64, max: i64 },
    Text { max_bytes: u16 },
    Content { kind: Kind },
}

impl FieldType {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            FieldType::Bool => Ok(()),
            FieldType::Integer { min, max } => {
                if min > max {
                    return Err("integer min > max".to_string());
                }
                Ok(())
            }
            FieldType::Text { max_bytes } => {
                if *max_bytes < 1 || *max_bytes > MAX_TEXT_BYTES {
                    return Err("text max_bytes out of range".to_string());
                }
                Ok(())
            }
            FieldType::Content { .. } => Ok(()),
        }
    }
}

pub type Components = BTreeMap<ContentId, BTreeMap<String, serde_json::Value>>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryStack {
    pub item: ContentId,
    pub count: u16,
}

impl InventoryStack {
    pub fn validate(&self) -> Result<(), String> {
        if self.count < 1 || self.count > MAX_STACK {
            return Err(format!("invalid stack count: {}", self.count));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "test", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    Always,
    ComponentEquals {
        component: ContentId,
        field: String,
        value: serde_json::Value,
    },
    InTag {
        tag: ContentId,
        content: ContentId,
    },
    All {
        conditions: Vec<ContentId>,
    },
    Any {
        conditions: Vec<ContentId>,
    },
    Not {
        condition: ContentId,
    },
}

impl Predicate {
    pub fn validate(&self) -> Result<(), String> {
        match self {
            Predicate::Always => Ok(()),
            Predicate::ComponentEquals { field, value, .. } => {
                validate_field_name(field)?;
                validate_json_value(value)?;
                Ok(())
            }
            Predicate::InTag { .. } => Ok(()),
            Predicate::All { conditions } => {
                if conditions.is_empty() || conditions.len() > MAX_CONDITION_REFS {
                    return Err("bad All arity".to_string());
                }
                let mut seen = BTreeSet::new();
                for c in conditions {
                    if !seen.insert(c) {
                        return Err("duplicate condition".to_string());
                    }
                }
                Ok(())
            }
            Predicate::Any { conditions } => {
                if conditions.is_empty() || conditions.len() > MAX_CONDITION_REFS {
                    return Err("bad Any arity".to_string());
                }
                let mut seen = BTreeSet::new();
                for c in conditions {
                    if !seen.insert(c) {
                        return Err("duplicate condition".to_string());
                    }
                }
                Ok(())
            }
            Predicate::Not { .. } => Ok(()),
        }
    }

    fn references(&self) -> Vec<(Kind, ContentId)> {
        match self {
            Predicate::Always => Vec::new(),
            Predicate::ComponentEquals { component, .. } => {
                vec![(Kind::Component, component.clone())]
            }
            Predicate::InTag { tag, .. } => vec![(Kind::Tag, tag.clone())],
            Predicate::All { conditions } | Predicate::Any { conditions } => conditions
                .iter()
                .map(|c| (Kind::Condition, c.clone()))
                .collect(),
            Predicate::Not { condition } => vec![(Kind::Condition, condition.clone())],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LootEntry {
    pub stack: InventoryStack,
    pub weight: u16,
}

impl LootEntry {
    pub fn validate(&self) -> Result<(), String> {
        self.stack.validate()?;
        if self.weight < 1 {
            return Err("loot weight must be positive".to_string());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContentSpec {
    Block {
        id: ContentId,
        states: StateSpec,
        model: ContentId,
        state_models: BTreeMap<String, ContentId>,
        collision: Vec<Box16>,
        selection: Vec<Box16>,
        opaque: bool,
        light: u8,
        hardness_milli: u32,
        components: Components,
    },
    Item {
        id: ContentId,
        model: ContentId,
        max_stack: u16,
        place_block: Option<ContentId>,
        components: Components,
    },
    Entity {
        id: ContentId,
        model: ContentId,
        dimensions_milli: [u16; 2],
        components: Components,
    },
    BlockEntity {
        id: ContentId,
        blocks: Vec<ContentId>,
        model: ContentId,
        components: Components,
    },
    Component {
        id: ContentId,
        version: u32,
        fields: BTreeMap<String, FieldType>,
    },
    Tag {
        id: ContentId,
        target: Kind,
        members: Vec<ContentId>,
        includes: Vec<ContentId>,
    },
    Recipe {
        id: ContentId,
        inputs: Vec<InventoryStack>,
        output: InventoryStack,
        condition: Option<ContentId>,
    },
    Loot {
        id: ContentId,
        entries: Vec<LootEntry>,
        condition: Option<ContentId>,
    },
    Condition {
        id: ContentId,
        predicate: Predicate,
    },
}

impl ContentSpec {
    pub fn id(&self) -> &ContentId {
        match self {
            ContentSpec::Block { id, .. }
            | ContentSpec::Item { id, .. }
            | ContentSpec::Entity { id, .. }
            | ContentSpec::BlockEntity { id, .. }
            | ContentSpec::Component { id, .. }
            | ContentSpec::Tag { id, .. }
            | ContentSpec::Recipe { id, .. }
            | ContentSpec::Loot { id, .. }
            | ContentSpec::Condition { id, .. } => id,
        }
    }

    pub fn kind(&self) -> Kind {
        match self {
            ContentSpec::Block { .. } => Kind::Block,
            ContentSpec::Item { .. } => Kind::Item,
            ContentSpec::Entity { .. } => Kind::Entity,
            ContentSpec::BlockEntity { .. } => Kind::BlockEntity,
            ContentSpec::Component { .. } => Kind::Component,
            ContentSpec::Tag { .. } => Kind::Tag,
            ContentSpec::Recipe { .. } => Kind::Recipe,
            ContentSpec::Loot { .. } => Kind::Loot,
            ContentSpec::Condition { .. } => Kind::Condition,
        }
    }

    pub fn component_defaults(&self) -> Option<&Components> {
        match self {
            ContentSpec::Block { components, .. }
            | ContentSpec::Item { components, .. }
            | ContentSpec::Entity { components, .. }
            | ContentSpec::BlockEntity { components, .. } => Some(components),
            _ => None,
        }
    }

    pub fn assets(&self) -> Vec<AssetRef> {
        let mut out = Vec::new();
        let push_model = |out: &mut Vec<AssetRef>, id: &ContentId| {
            out.push(AssetRef {
                kind: AssetKind::Model,
                id: id.clone(),
            });
        };
        match self {
            ContentSpec::Block {
                model,
                state_models,
                ..
            } => {
                push_model(&mut out, model);
                for m in state_models.values() {
                    push_model(&mut out, m);
                }
            }
            ContentSpec::Item { model, .. }
            | ContentSpec::Entity { model, .. }
            | ContentSpec::BlockEntity { model, .. } => push_model(&mut out, model),
            _ => {}
        }
        out
    }

    pub fn references(&self) -> Vec<(Kind, ContentId)> {
        let mut out = Vec::new();
        match self {
            ContentSpec::Block { components, .. } => {
                for k in components.keys() {
                    out.push((Kind::Component, k.clone()));
                }
            }
            ContentSpec::Item {
                place_block,
                components,
                ..
            } => {
                if let Some(b) = place_block {
                    out.push((Kind::Block, b.clone()));
                }
                for k in components.keys() {
                    out.push((Kind::Component, k.clone()));
                }
            }
            ContentSpec::Entity { components, .. } => {
                for k in components.keys() {
                    out.push((Kind::Component, k.clone()));
                }
            }
            ContentSpec::BlockEntity {
                blocks, components, ..
            } => {
                for b in blocks {
                    out.push((Kind::Block, b.clone()));
                }
                for k in components.keys() {
                    out.push((Kind::Component, k.clone()));
                }
            }
            ContentSpec::Component { .. } => {}
            ContentSpec::Tag {
                target,
                members,
                includes,
                ..
            } => {
                for m in members {
                    out.push((*target, m.clone()));
                }
                for i in includes {
                    out.push((Kind::Tag, i.clone()));
                }
            }
            ContentSpec::Recipe {
                inputs,
                output,
                condition,
                ..
            } => {
                for s in inputs {
                    out.push((Kind::Item, s.item.clone()));
                }
                out.push((Kind::Item, output.item.clone()));
                if let Some(c) = condition {
                    out.push((Kind::Condition, c.clone()));
                }
            }
            ContentSpec::Loot {
                entries, condition, ..
            } => {
                for e in entries {
                    out.push((Kind::Item, e.stack.item.clone()));
                }
                if let Some(c) = condition {
                    out.push((Kind::Condition, c.clone()));
                }
            }
            ContentSpec::Condition { predicate, .. } => out.extend(predicate.references()),
        }
        out
    }

    pub fn validate_local(&self) -> Result<(), String> {
        match self {
            ContentSpec::Block {
                states,
                state_models,
                collision,
                selection,
                light,
                hardness_milli,
                components,
                ..
            } => {
                validate_components(components)?;
                states.schema().map(|_| ())?;
                if state_models.len() > MAX_STATE_MODELS {
                    return Err("too many state models".to_string());
                }
                if collision.len() > MAX_BOXES {
                    return Err("too many collision boxes".to_string());
                }
                if selection.is_empty() || selection.len() > MAX_BOXES {
                    return Err("bad selection box count".to_string());
                }
                for b in collision.iter().chain(selection.iter()) {
                    b.validate()?;
                }
                if *light > MAX_LIGHT {
                    return Err("light out of range".to_string());
                }
                if *hardness_milli > MAX_HARDNESS_MILLI {
                    return Err("hardness out of range".to_string());
                }
                Ok(())
            }
            ContentSpec::Item {
                max_stack,
                components,
                ..
            } => {
                if *max_stack < 1 || *max_stack > MAX_STACK {
                    return Err("max_stack out of range".to_string());
                }
                validate_components(components)?;
                Ok(())
            }
            ContentSpec::Entity {
                dimensions_milli,
                components,
                ..
            } => {
                for d in dimensions_milli {
                    if *d < 1 || *d > MAX_ENTITY_DIM_MILLI {
                        return Err("entity dimension out of range".to_string());
                    }
                }
                validate_components(components)?;
                Ok(())
            }
            ContentSpec::BlockEntity {
                blocks, components, ..
            } => {
                if blocks.is_empty() || blocks.len() > MAX_BLOCKENTITY_BLOCKS {
                    return Err("bad block entity block count".to_string());
                }
                let mut seen = BTreeSet::new();
                for b in blocks {
                    if !seen.insert(b) {
                        return Err("duplicate block entity block".to_string());
                    }
                }
                validate_components(components)?;
                Ok(())
            }
            ContentSpec::Component {
                version, fields, ..
            } => {
                if *version < 1 {
                    return Err("component version must be positive".to_string());
                }
                if fields.is_empty() || fields.len() > MAX_COMPONENT_FIELDS {
                    return Err("bad component field count".to_string());
                }
                for (k, t) in fields {
                    validate_field_name(k)?;
                    t.validate()?;
                }
                Ok(())
            }
            ContentSpec::Tag {
                target,
                members,
                includes,
                ..
            } => {
                if *target == Kind::Tag {
                    return Err("tag target cannot be tag".to_string());
                }
                if members.len() > MAX_TAG_MEMBERS {
                    return Err("too many tag members".to_string());
                }
                if includes.len() > MAX_TAG_INCLUDES {
                    return Err("too many tag includes".to_string());
                }
                let mut seen = BTreeSet::new();
                for m in members {
                    if !seen.insert(m) {
                        return Err("duplicate tag member".to_string());
                    }
                }
                let mut seen2 = BTreeSet::new();
                for i in includes {
                    if !seen2.insert(i) {
                        return Err("duplicate tag include".to_string());
                    }
                }
                Ok(())
            }
            ContentSpec::Recipe { inputs, output, .. } => {
                if inputs.is_empty() || inputs.len() > MAX_RECIPE_INPUTS {
                    return Err("bad recipe input count".to_string());
                }
                let mut seen = BTreeSet::new();
                for s in inputs {
                    s.validate()?;
                    if !seen.insert(&s.item) {
                        return Err("duplicate recipe input item".to_string());
                    }
                }
                output.validate()?;
                Ok(())
            }
            ContentSpec::Loot { entries, .. } => {
                if entries.is_empty() || entries.len() > MAX_LOOT_ENTRIES {
                    return Err("bad loot entry count".to_string());
                }
                for e in entries {
                    e.validate()?;
                }
                Ok(())
            }
            ContentSpec::Condition { predicate, .. } => predicate.validate(),
        }
    }
}
