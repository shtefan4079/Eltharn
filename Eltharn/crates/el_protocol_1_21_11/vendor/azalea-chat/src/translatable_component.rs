use std::fmt::{self, Display};

use serde::{Deserialize, Serialize};
#[cfg(feature = "simdnbt")]
use simdnbt::{
    ToNbtTag,
    owned::{NbtList, NbtTag},
};

use crate::{FormattedText, base_component::BaseComponent, text_component::TextComponent};

fn java_literal_component(text: impl Into<String>) -> TextComponent {
    TextComponent::new(text)
}

fn java_literal_formatted_text(text: impl Into<String>) -> FormattedText {
    FormattedText::Text(java_literal_component(text))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TranslationFormat {
    Percent { end: usize },
    String { position: Option<usize>, end: usize },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum PrimitiveOrComponent {
    Boolean(bool),
    Short(i16),
    Integer(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(String),
    FormattedText(FormattedText),
}

#[cfg(feature = "simdnbt")]
impl simdnbt::ToNbtTag for PrimitiveOrComponent {
    fn to_nbt_tag(self) -> simdnbt::owned::NbtTag {
        match self {
            PrimitiveOrComponent::Boolean(value) => value.to_nbt_tag(),
            PrimitiveOrComponent::Short(value) => value.to_nbt_tag(),
            PrimitiveOrComponent::Integer(value) => value.to_nbt_tag(),
            PrimitiveOrComponent::Long(value) => value.to_nbt_tag(),
            PrimitiveOrComponent::Float(value) => value.to_nbt_tag(),
            PrimitiveOrComponent::Double(value) => value.to_nbt_tag(),
            PrimitiveOrComponent::String(value) => value.to_nbt_tag(),
            PrimitiveOrComponent::FormattedText(value) => value.to_nbt_tag(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TranslatableComponent {
    #[serde(flatten)]
    pub base: BaseComponent,
    #[serde(rename = "translate")]
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback: Option<String>,
    #[serde(rename = "with")]
    pub args: Vec<PrimitiveOrComponent>,
}

#[cfg(feature = "simdnbt")]
fn serialize_args_as_nbt(args: Vec<PrimitiveOrComponent>) -> NbtList {
    let tags = args
        .into_iter()
        .map(|arg| arg.to_nbt_tag())
        .collect::<Vec<NbtTag>>();
    NbtList::from(tags)
}

#[cfg(feature = "simdnbt")]
impl simdnbt::Serialize for TranslatableComponent {
    fn to_compound(self) -> simdnbt::owned::NbtCompound {
        let mut compound = simdnbt::owned::NbtCompound::new();
        compound.insert("translate", self.key);
        if let Some(fallback) = self.fallback {
            compound.insert("fallback", fallback);
        }
        compound.extend(self.base.to_compound());
        compound.insert("with", serialize_args_as_nbt(self.args));
        compound
    }
}

impl TranslatableComponent {
    pub fn new(key: String, args: Vec<PrimitiveOrComponent>) -> Self {
        Self {
            base: BaseComponent::new(),
            key,
            fallback: None,
            args,
        }
    }

    pub fn with_fallback(
        key: String,
        fallback: Option<String>,
        args: Vec<PrimitiveOrComponent>,
    ) -> Self {
        Self {
            base: BaseComponent::new(),
            key,
            fallback,
            args,
        }
    }

    pub fn read(&self) -> Result<TextComponent, fmt::Error> {
        let template = azalea_language::get(&self.key).unwrap_or_else(|| {
            if let Some(fallback) = &self.fallback {
                fallback.as_str()
            } else {
                &self.key
            }
        });

        let parts = self
            .decompose_template(template)
            .unwrap_or_else(|_| vec![java_literal_formatted_text(template)]);

        Ok(TextComponent {
            base: BaseComponent {
                siblings: parts,
                style: Default::default(),
                raw_interaction_probe: None,
            },
            text: String::new(),
        })
    }

    fn decompose_template(&self, template: &str) -> Result<Vec<FormattedText>, ()> {
        let bytes = template.as_bytes();
        let mut parts = Vec::new();
        let mut literal_start = 0usize;
        let mut unpositioned_argument = 0usize;
        let mut cursor = 0usize;

        while cursor < bytes.len() {
            if bytes[cursor] != b'%' {
                cursor += 1;
                continue;
            }

            let format = Self::parse_format(template, cursor)?;
            if literal_start < cursor {
                parts.push(java_literal_formatted_text(
                    &template[literal_start..cursor],
                ));
            }

            match format {
                TranslationFormat::Percent { end } => {
                    parts.push(java_literal_formatted_text("%"));
                    cursor = end;
                }
                TranslationFormat::String { position, end } => {
                    let index = position.unwrap_or_else(|| {
                        let current = unpositioned_argument;
                        unpositioned_argument += 1;
                        current
                    });
                    parts.push(self.argument(index)?);
                    cursor = end;
                }
            }
            literal_start = cursor;
        }

        if literal_start < template.len() {
            parts.push(java_literal_formatted_text(&template[literal_start..]));
        }

        Ok(parts)
    }

    fn parse_format(template: &str, start: usize) -> Result<TranslationFormat, ()> {
        let bytes = template.as_bytes();
        debug_assert_eq!(bytes.get(start), Some(&b'%'));

        let after_percent = start + 1;
        if after_percent == bytes.len() {
            return Err(());
        }

        let mut digits_end = after_percent;
        while matches!(bytes.get(digits_end), Some(b'0'..=b'9')) {
            digits_end += 1;
        }

        let (position, type_index) =
            if digits_end > after_percent && bytes.get(digits_end) == Some(&b'$') {
                let one_based = template[after_percent..digits_end]
                    .parse::<i32>()
                    .map_err(|_| ())?;
                let zero_based = one_based.checked_sub(1).ok_or(())?;
                let position = usize::try_from(zero_based).map_err(|_| ())?;
                (Some(position), digits_end + 1)
            } else {
                (None, after_percent)
            };

        match bytes.get(type_index) {
            Some(b'%') if position.is_none() => Ok(TranslationFormat::Percent {
                end: type_index + 1,
            }),
            Some(b's') => Ok(TranslationFormat::String {
                position,
                end: type_index + 1,
            }),
            _ => Err(()),
        }
    }

    fn argument(&self, index: usize) -> Result<FormattedText, ()> {
        let argument = self.args.get(index).ok_or(())?;
        Ok(match argument {
            PrimitiveOrComponent::FormattedText(component) => component.clone(),
            primitive => java_literal_formatted_text(primitive.to_string()),
        })
    }
}

impl Display for TranslatableComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for component in FormattedText::Translatable(self.clone()).into_iter() {
            let component_text = match &component {
                FormattedText::Text(c) => c.text.to_string(),
                FormattedText::Translatable(c) => match c.read() {
                    Ok(c) => c.to_string(),
                    Err(_) => c.key.to_string(),
                },
            };

            f.write_str(&component_text)?;
        }

        Ok(())
    }
}

impl Display for PrimitiveOrComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            PrimitiveOrComponent::Boolean(value) => write!(f, "{value}"),
            PrimitiveOrComponent::Short(value) => write!(f, "{value}"),
            PrimitiveOrComponent::Integer(value) => write!(f, "{value}"),
            PrimitiveOrComponent::Long(value) => write!(f, "{value}"),
            PrimitiveOrComponent::Float(value) => write!(f, "{value}"),
            PrimitiveOrComponent::Double(value) => write!(f, "{value}"),
            PrimitiveOrComponent::String(value) => write!(f, "{value}"),
            PrimitiveOrComponent::FormattedText(value) => write!(f, "{value}"),
        }
    }
}

impl From<PrimitiveOrComponent> for TextComponent {
    fn from(soc: PrimitiveOrComponent) -> Self {
        match soc {
            PrimitiveOrComponent::String(value) => java_literal_component(value),
            PrimitiveOrComponent::Boolean(value) => java_literal_component(value.to_string()),
            PrimitiveOrComponent::Short(value) => java_literal_component(value.to_string()),
            PrimitiveOrComponent::Integer(value) => java_literal_component(value.to_string()),
            PrimitiveOrComponent::Long(value) => java_literal_component(value.to_string()),
            PrimitiveOrComponent::Float(value) => java_literal_component(value.to_string()),
            PrimitiveOrComponent::Double(value) => java_literal_component(value.to_string()),
            PrimitiveOrComponent::FormattedText(value) => TextComponent {
                base: BaseComponent {
                    siblings: vec![value],
                    style: Default::default(),
                    raw_interaction_probe: None,
                },
                text: String::new(),
            },
        }
    }
}
impl From<&str> for TranslatableComponent {
    fn from(s: &str) -> Self {
        TranslatableComponent::new(s.to_owned(), vec![])
    }
}


