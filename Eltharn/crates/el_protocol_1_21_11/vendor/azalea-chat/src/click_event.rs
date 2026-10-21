use serde::Serialize;
#[cfg(feature = "simdnbt")]
use simdnbt::{
    DeserializeError,
    owned::{NbtCompound, NbtTag},
};

#[cfg(feature = "simdnbt")]
use crate::get_in_compound;

macro_rules! define_click_event_struct {
    (
        $(
            $action_name:ident : $action_variant:ident {
                $(
                    $(#[$meta:meta])* $field:ident : $type:ty
                ),*
                $(,)?
            }
        ),*
        $(,)?
    ) => {
        #[derive(Clone, Debug, PartialEq, Serialize)]
        #[serde(rename_all = "snake_case", tag = "action")]
        pub enum ClickEvent {
            $(
                $action_variant {
                    $(
                        $(#[$meta])*
                        $field: $type
                    ),*
                }
            ),*
        }

        #[cfg(feature = "simdnbt")]
        impl simdnbt::Serialize for ClickEvent {
            fn to_compound(self) -> NbtCompound {
                let mut compound = NbtCompound::new();
                match self {
                    $(
                        Self::$action_variant { $($field),* } => {
                            compound.insert("action", stringify!($action_name));
                            $(
                                if let Some(tag) = simdnbt::ToNbtTag::to_optional_nbt_tag($field) {
                                    compound.insert(stringify!($field), tag);
                                }
                            )*
                        }
                    )*
                };
                compound
            }
        }

        #[cfg(feature = "simdnbt")]
        impl simdnbt::Deserialize for ClickEvent {
            fn from_compound(
                compound: simdnbt::borrow::NbtCompound,
            ) -> Result<Self, simdnbt::DeserializeError> {
                let action = get_in_compound::<String>(&compound, "action")?;
                Ok(match action.as_str() {
                    $(
                        stringify!($action_name) => Self::$action_variant {
                            $(
                                $field: get_in_compound(&compound, stringify!($field))?
                            ),*
                        },
                    )*
                    _ => return Err(DeserializeError::MismatchedFieldType(action.to_owned())),
                })
            }
        }

    }
}

impl ClickEvent {
    pub fn from_json(value: &serde_json::Value) -> Option<Self> {
        let action = value.get("action")?.as_str()?;
        let string = |field: &str| {
            value
                .get(field)
                .or_else(|| value.get("value"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        };
        Some(match action {
            "open_url" => Self::OpenUrl {
                url: string("url")?,
            },
            "open_file" => Self::OpenFile {
                path: string("path")?,
            },
            "run_command" => Self::RunCommand {
                command: string("command")?,
            },
            "suggest_command" => Self::SuggestCommand {
                command: string("command")?,
            },
            "copy_to_clipboard" => Self::CopyToClipboard {
                value: string("value")?,
            },
            "change_page" => Self::ChangePage {
                page: value
                    .get("page")
                    .and_then(serde_json::Value::as_i64)
                    .map(|page| page as i32)
                    .or_else(|| string("value")?.parse().ok())?,
            },
            "show_dialog" => Self::ShowDialog {},
            "custom" => Self::Custom {
                id: value.get("id")?.as_str()?.to_owned(),
                #[cfg(feature = "simdnbt")]
                payload: value.get("payload").and_then(crate::hover_event::json_tag),
            },
            _ => return None,
        })
    }
}

define_click_event_struct! {
    open_url: OpenUrl {
        url: String,
    },
    open_file: OpenFile {
        path: String,
    },
    run_command: RunCommand {
        command: String,
    },
    suggest_command: SuggestCommand {
        command: String,
    },
    show_dialog: ShowDialog {},
    change_page: ChangePage {
        page: i32,
    },
    copy_to_clipboard: CopyToClipboard {
        value: String,
    },
    custom: Custom {
        id: String,
        #[cfg(feature = "simdnbt")]
        payload: Option<NbtTag>,
    },
}

#[cfg(feature = "simdnbt")]
impl ClickEvent {
    pub(crate) fn from_legacy_compound(
        compound: simdnbt::borrow::NbtCompound,
    ) -> Result<Self, DeserializeError> {
        let action = get_in_compound::<String>(&compound, "action")?;
        let value = get_in_compound::<String>(&compound, "value")?;

        Ok(match action.as_str() {
            "open_url" => Self::OpenUrl { url: value },
            "open_file" => Self::OpenFile { path: value },
            "run_command" => Self::RunCommand { command: value },
            "suggest_command" => Self::SuggestCommand { command: value },
            "change_page" => Self::ChangePage {
                page: value
                    .parse()
                    .map_err(|_| DeserializeError::MismatchedFieldType(value.clone()))?,
            },
            "copy_to_clipboard" => Self::CopyToClipboard { value },
            _ => return Err(DeserializeError::MismatchedFieldType(action)),
        })
    }
}
