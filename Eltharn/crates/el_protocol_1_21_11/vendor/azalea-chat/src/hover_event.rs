use crate::FormattedText;
use serde::Serialize;
#[cfg(feature = "simdnbt")]
use simdnbt::{DeserializeError, FromNbtTag, owned::NbtCompound};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum HoverEvent {
    ShowText {
        value: Box<FormattedText>,
    },
    ShowItem {
        id: String,
        count: u32,
        #[cfg(feature = "simdnbt")]
        components: Option<NbtCompound>,
    },
    ShowEntity {
        id: String,
        uuid: String,
        name: Option<Box<FormattedText>>,
    },
}

fn json_uuid(value: &serde_json::Value) -> Option<String> {
    if let Some(value) = value.as_str() {
        return Some(value.to_owned());
    }
    let words = value.as_array()?;
    if words.len() != 4 {
        return None;
    }
    let mut bits = 0_u128;
    for word in words {
        bits = (bits << 32) | (word.as_i64()? as u32 as u128);
    }
    Some(format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        bits >> 96,
        (bits >> 80) & 65535,
        (bits >> 64) & 65535,
        (bits >> 48) & 65535,
        bits & 0xffffffffffff
    ))
}

impl HoverEvent {
    pub fn from_json(json: &serde_json::Value) -> Option<Self> {
        let action = json.get("action")?.as_str()?;
        let data = json
            .get("contents")
            .or_else(|| json.get("value"))
            .unwrap_or(json);
        match action {
            "show_text" => Some(Self::ShowText {
                value: Box::new(serde_json::from_value(data.clone()).ok()?),
            }),
            "show_item" => Some(Self::ShowItem {
                id: data.get("id")?.as_str()?.to_owned(),
                count: data
                    .get("count")
                    .and_then(|value| value.as_u64())
                    .unwrap_or(1)
                    .clamp(1, 99) as u32,
                #[cfg(feature = "simdnbt")]
                components: data.get("components").and_then(json_compound),
            }),
            "show_entity" => Some(Self::ShowEntity {
                id: data
                    .get("type")
                    .or_else(|| data.get("id"))?
                    .as_str()?
                    .to_owned(),
                uuid: json_uuid(
                    data.get("uuid")
                        .or_else(|| data.get("id").filter(|_| data.get("type").is_some()))?,
                )?,
                name: data
                    .get("name")
                    .and_then(|value| serde_json::from_value(value.clone()).ok())
                    .map(Box::new),
            }),
            _ => None,
        }
    }
}

#[cfg(feature = "simdnbt")]
fn json_compound(value: &serde_json::Value) -> Option<NbtCompound> {
    let mut compound = NbtCompound::new();
    for (key, value) in value.as_object()? {
        compound.insert(key.as_str(), json_tag(value)?);
    }
    Some(compound)
}

#[cfg(feature = "simdnbt")]
pub(crate) fn json_tag(value: &serde_json::Value) -> Option<simdnbt::owned::NbtTag> {
    use simdnbt::owned::{NbtList, NbtTag};
    Some(match value {
        serde_json::Value::Bool(value) => NbtTag::Byte(i8::from(*value)),
        serde_json::Value::Number(value) if value.is_i64() => {
            let integer = value.as_i64()?;
            if let Ok(integer) = i32::try_from(integer) {
                NbtTag::Int(integer)
            } else {
                NbtTag::Long(integer)
            }
        }
        serde_json::Value::Number(value) => NbtTag::Double(value.as_f64()?),
        serde_json::Value::String(value) => NbtTag::String(value.as_str().into()),
        serde_json::Value::Object(_) => NbtTag::Compound(json_compound(value)?),
        serde_json::Value::Array(values) if values.is_empty() => NbtTag::List(NbtList::Empty),
        serde_json::Value::Array(values) if values.iter().all(|value| value.is_string()) => {
            NbtTag::List(NbtList::String(
                values
                    .iter()
                    .map(|value| value.as_str().unwrap().into())
                    .collect(),
            ))
        }
        serde_json::Value::Array(values) if values.iter().all(|value| value.is_object()) => {
            NbtTag::List(NbtList::Compound(
                values
                    .iter()
                    .map(json_compound)
                    .collect::<Option<Vec<_>>>()?,
            ))
        }
        serde_json::Value::Array(values) if values.iter().all(|value| value.is_i64()) => {
            NbtTag::List(NbtList::Int(
                values
                    .iter()
                    .map(|value| value.as_i64().unwrap() as i32)
                    .collect(),
            ))
        }
        _ => return None,
    })
}

#[cfg(feature = "simdnbt")]
impl simdnbt::Deserialize for HoverEvent {
    fn from_compound(compound: simdnbt::borrow::NbtCompound) -> Result<Self, DeserializeError> {
        let action = compound
            .string("action")
            .ok_or(DeserializeError::MissingField)?
            .to_str();
        let data = compound
            .compound("contents")
            .or_else(|| compound.compound("value"))
            .unwrap_or(compound);
        match action.as_ref() {
            "show_text" => {
                let value = compound
                    .get("value")
                    .or_else(|| compound.get("contents"))
                    .ok_or(DeserializeError::MissingField)?;
                Ok(Self::ShowText {
                    value: Box::new(
                        FormattedText::from_nbt_tag(value).ok_or(DeserializeError::MissingField)?,
                    ),
                })
            }
            "show_item" => Ok(Self::ShowItem {
                id: data
                    .string("id")
                    .ok_or(DeserializeError::MissingField)?
                    .to_string(),
                count: data
                    .int("count")
                    .or_else(|| data.byte("count").map(i32::from))
                    .unwrap_or(1)
                    .clamp(1, 99) as u32,
                components: data.compound("components").map(|value| value.to_owned()),
            }),
            "show_entity" => {
                let id = data
                    .string("type")
                    .or_else(|| data.string("id"))
                    .ok_or(DeserializeError::MissingField)?
                    .to_string();
                let uuid = data
                    .string("uuid")
                    .map(|value| value.to_string())
                    .or_else(|| {
                        data.int_array("uuid")
                            .and_then(|words| json_uuid(&serde_json::json!(words)))
                    })
                    .or_else(|| {
                        data.string("id")
                            .filter(|_| data.contains("type"))
                            .map(|value| value.to_string())
                    })
                    .ok_or(DeserializeError::MissingField)?;
                Ok(Self::ShowEntity {
                    id,
                    uuid,
                    name: data
                        .get("name")
                        .and_then(FormattedText::from_nbt_tag)
                        .map(Box::new),
                })
            }
            _ => Err(DeserializeError::MismatchedFieldType(action.to_string())),
        }
    }
}

#[cfg(feature = "simdnbt")]
impl simdnbt::Serialize for HoverEvent {
    fn to_compound(self) -> NbtCompound {
        let mut compound = NbtCompound::new();
        match self {
            Self::ShowText { value } => {
                compound.insert("action", "show_text");
                compound.insert("value", value.to_compound());
            }
            Self::ShowItem {
                id,
                count,
                components,
            } => {
                compound.insert("action", "show_item");
                compound.insert("id", id);
                compound.insert("count", count as i32);
                if let Some(components) = components {
                    compound.insert("components", components);
                }
            }
            Self::ShowEntity { id, uuid, name } => {
                compound.insert("action", "show_entity");
                compound.insert("id", id);
                compound.insert("uuid", uuid);
                if let Some(name) = name {
                    compound.insert("name", name.to_compound());
                }
            }
        }
        compound
    }
}
