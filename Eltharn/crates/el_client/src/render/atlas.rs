use crate::resources::manager::{ClientResourceManager, ResourceError};
use el_core::resource_location::ResourceLocation;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct AtlasDefinition {
    pub sources: Vec<SpriteSourceDefinition>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct SpriteSourceDefinition {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(flatten)]
    pub data: BTreeMap<String, Value>,
}
#[derive(Debug)]
pub enum AtlasLoadError {
    Resource(ResourceError),
    Json {
        id: ResourceLocation,
        source: serde_json::Error,
    },
    MissingField {
        source_type: String,
        field: &'static str,
    },
}
impl AtlasDefinition {
    pub fn load(
        resources: &ClientResourceManager,
        id: &ResourceLocation,
    ) -> Result<Self, AtlasLoadError> {
        let definitions = resources
            .read_atlas_stack(id)
            .map_err(AtlasLoadError::Resource)?;
        let mut sources = Vec::new();
        let mut first_parse_error = None;
        for text in definitions {
            match serde_json::from_str::<AtlasDefinition>(&text) {
                Ok(definition) => sources.extend(definition.sources),
                Err(source) => {
                    if first_parse_error.is_none() {
                        first_parse_error = Some(source);
                    }
                }
            }
        }
        if sources.is_empty() {
            if let Some(source) = first_parse_error {
                return Err(AtlasLoadError::Json {
                    id: id.clone(),
                    source,
                });
            }
        }
        Ok(Self { sources })
    }
    pub fn collect_sprite_ids(
        &self,
        resources: &ClientResourceManager,
        namespace: &str,
    ) -> Result<Vec<ResourceLocation>, AtlasLoadError> {
        let mut sprites = Vec::new();
        for source in &self.sources {
            match source.source_type.as_str() {
                "minecraft:directory" => {
                    let source_path = source.required_string("source")?;
                    let prefix = source.required_string("prefix")?;
                    sprites.extend(
                        resources
                            .list_texture_ids_in_directory(namespace, source_path, prefix)
                            .map_err(AtlasLoadError::Resource)?,
                    );
                }
                "minecraft:single" => {
                    let resource = source.required_string("resource")?;
                    let sprite = source.optional_string("sprite").unwrap_or(resource);
                    sprites.push(ResourceLocation::from_string(sprite));
                }
                _ => {}
            }
        }
        sprites.sort_by(|left, right| left.to_string().cmp(&right.to_string()));
        sprites.dedup_by(|left, right| left == right);
        Ok(sprites)
    }
}
impl SpriteSourceDefinition {
    pub fn required_string(&self, field: &'static str) -> Result<&str, AtlasLoadError> {
        self.optional_string(field)
            .ok_or_else(|| AtlasLoadError::MissingField {
                source_type: self.source_type.clone(),
                field,
            })
    }
    pub fn optional_string(&self, field: &'static str) -> Option<&str> {
        self.data.get(field).and_then(Value::as_str)
    }
}
impl fmt::Display for AtlasLoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AtlasLoadError::Resource(error) => write!(f, "{}", error),
            AtlasLoadError::Json { id, source } => write!(f, "{}: {}", id, source),
            AtlasLoadError::MissingField { source_type, field } => {
                write!(f, "{source_type} source is missing field {field}")
            }
        }
    }
}
impl std::error::Error for AtlasLoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AtlasLoadError::Resource(error) => Some(error),
            AtlasLoadError::Json { source, .. } => Some(source),
            AtlasLoadError::MissingField { .. } => None,
        }
    }
}
