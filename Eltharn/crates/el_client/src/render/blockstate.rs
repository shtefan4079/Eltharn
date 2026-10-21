#![allow(dead_code)]
use crate::resources::manager::{ClientResourceManager, ResourceError};
use el_core::block_state::BlockState;
use el_core::resource_location::ResourceLocation;
use serde::Deserialize;
use std::collections::HashMap;
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum VariantValue {
    Single(BlockStateVariant),
    Multiple(Vec<BlockStateVariant>),
}
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct BlockStateVariant {
    pub model: String,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub z: Option<i32>,
    pub uvlock: Option<bool>,
    pub weight: Option<i32>,
}
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct BlockStateDefinition {
    pub variants: Option<HashMap<String, VariantValue>>,
    pub multipart: Option<Vec<MultipartRule>>,
}
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct MultipartRule {
    pub when: Option<MultipartCondition>,
    pub apply: VariantValue,
}
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(untagged)]
pub enum MultipartCondition {
    Or {
        #[serde(rename = "OR")]
        or: Vec<MultipartCondition>,
    },
    And {
        #[serde(rename = "AND")]
        and: Vec<MultipartCondition>,
    },
    Single(HashMap<String, String>),
}
impl BlockStateDefinition {
    pub fn load(
        resources: &ClientResourceManager,
        id: &ResourceLocation,
    ) -> Result<Self, ResourceError> {
        let text = resources.read_blockstate(id)?;
        let def: BlockStateDefinition =
            serde_json::from_str(&text).map_err(|e| ResourceError::Parse {
                id: id.clone(),
                message: e.to_string(),
            })?;
        def.warn_on_unsupported_z_rotation(id);
        Ok(def)
    }
    fn warn_on_unsupported_z_rotation(&self, id: &ResourceLocation) {
        let variant_uses_z =
            |value: &VariantValue| value.all_variants().iter().any(|v| v.z.unwrap_or(0) != 0);
        let uses_z = self
            .variants
            .as_ref()
            .is_some_and(|variants| variants.values().any(variant_uses_z))
            || self
                .multipart
                .as_ref()
                .is_some_and(|rules| rules.iter().any(|rule| variant_uses_z(&rule.apply)));
        if uses_z {
            static WARNED: std::sync::Once = std::sync::Once::new();
            WARNED
                .call_once(|| {
                    log::warn!(
                        "Blockstate {id} uses a `z` variant rotation; Eltharn currently composes only x and y rotations"
                    );
                });
        }
    }
    pub fn select_variant_groups(&self, state: &BlockState) -> Vec<Vec<BlockStateVariant>> {
        self.select_variant_groups_with_random_mode(state)
            .0
            .into_iter()
            .map(|value| value.all_variants().to_vec())
            .collect()
    }
    pub fn select_variant_groups_with_random_mode(
        &self,
        state: &BlockState,
    ) -> (Vec<VariantValue>, bool) {
        let properties = effective_properties(state);
        if let Some(variants) = &self.variants {
            let mut entries: Vec<_> = variants.iter().collect();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            for (key, value) in entries {
                if variant_key_matches(key, &properties) {
                    return (vec![value.clone()], false);
                }
            }
        }
        let mut resolved = Vec::new();
        if let Some(multipart) = &self.multipart {
            for rule in multipart {
                let matches = rule
                    .when
                    .as_ref()
                    .map(|condition| multipart_condition_matches(condition, &properties))
                    .unwrap_or(true);
                if matches {
                    resolved.push(rule.apply.clone());
                }
            }
        }
        (resolved, self.multipart.is_some())
    }
}
impl VariantValue {
    fn all_variants(&self) -> &[BlockStateVariant] {
        match self {
            Self::Single(variant) => std::slice::from_ref(variant),
            Self::Multiple(variants) => variants,
        }
    }
}
pub fn effective_properties(state: &BlockState) -> HashMap<String, String> {
    let mut properties = state.get_block().default_state().properties.clone();
    properties.extend(state.properties.clone());
    properties
}
fn variant_key_matches(key: &str, properties: &HashMap<String, String>) -> bool {
    if key.is_empty() {
        return true;
    }
    key.split(',').all(|condition| {
        let Some((name, expected)) = condition.split_once('=') else {
            return false;
        };
        property_value_matches(properties.get(name), expected)
    })
}
fn multipart_condition_matches(
    condition: &MultipartCondition,
    properties: &HashMap<String, String>,
) -> bool {
    match condition {
        MultipartCondition::Or { or } => or
            .iter()
            .any(|child| multipart_condition_matches(child, properties)),
        MultipartCondition::And { and } => and
            .iter()
            .all(|child| multipart_condition_matches(child, properties)),
        MultipartCondition::Single(values) => values
            .iter()
            .all(|(name, expected)| property_value_matches(properties.get(name), expected)),
    }
}
fn property_value_matches(actual: Option<&String>, expected: &str) -> bool {
    let Some(actual) = actual else {
        return false;
    };
    expected.split('|').any(|candidate| candidate == actual)
}
