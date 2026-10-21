use std::fmt::{self, Display};

use serde::{Serialize, Serializer, ser::SerializeMap};

use crate::{
    FormattedText,
    base_component::BaseComponent,
    style::{ChatFormatting, Style, TextColor},
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextComponent {
    pub base: BaseComponent,
    pub text: String,
}
impl Serialize for TextComponent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if self.base == BaseComponent::default() {
            return serializer.serialize_str(&self.text);
        }

        let mut state = serializer.serialize_map(None)?;
        state.serialize_entry("text", &self.text)?;

        self.base.serialize_map::<S>(&mut state)?;

        state.end()
    }
}

#[cfg(feature = "simdnbt")]
impl simdnbt::Serialize for TextComponent {
    fn to_compound(self) -> simdnbt::owned::NbtCompound {
        let mut compound = simdnbt::owned::NbtCompound::new();
        compound.insert("text", self.text);
        compound.extend(self.base.style.to_compound());
        if !self.base.siblings.is_empty() {
            compound.insert(
                "extra",
                simdnbt::owned::NbtList::from(
                    self.base
                        .siblings
                        .into_iter()
                        .map(|component| component.to_compound())
                        .collect::<Vec<_>>(),
                ),
            );
        }
        compound
    }
}

const LEGACY_FORMATTING_CODE_SYMBOL: char = '§';

pub fn legacy_color_code_to_text_component(legacy_color_code: &str) -> TextComponent {
    if legacy_color_code.is_empty() {
        return TextComponent::new("");
    }

    let mut components: Vec<TextComponent> = Vec::with_capacity(1);

    let mut cur_component = TextComponent::new("");

    let mut i = 0;
    while i < legacy_color_code.chars().count() {
        if legacy_color_code.chars().nth(i).unwrap() == LEGACY_FORMATTING_CODE_SYMBOL {
            let formatting_code = legacy_color_code.chars().nth(i + 1);
            let Some(formatting_code) = formatting_code else {
                i += 1;
                continue;
            };
            if formatting_code == '#' {
                let color = legacy_color_code
                    .chars()
                    .skip(i + 1)
                    .take(7)
                    .collect::<String>();

                if !cur_component.text.is_empty() {
                    components.push(cur_component.clone());
                    cur_component.text = "".to_owned();
                };
                cur_component.base.style.color = TextColor::parse(&color);

                i += 6;
            } else if let Some(formatter) = ChatFormatting::from_code(formatting_code) {
                if !cur_component.text.is_empty() || formatter == ChatFormatting::Reset {
                    components.push(cur_component.clone());
                    cur_component.text = "".to_owned();
                };
                cur_component.base.style.apply_formatting(&formatter);
            }
            i += 1;
        } else {
            cur_component
                .text
                .push(legacy_color_code.chars().nth(i).unwrap());
        };
        i += 1;
    }

    components.push(cur_component);

    let mut final_component = TextComponent::new("");
    for component in components {
        final_component.base.siblings.push(component.get());
    }

    final_component
}

impl TextComponent {
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        if text.contains(LEGACY_FORMATTING_CODE_SYMBOL) {
            legacy_color_code_to_text_component(&text)
        } else {
            Self {
                base: BaseComponent::new(),
                text,
            }
        }
    }

    fn get(self) -> FormattedText {
        FormattedText::Text(self)
    }
    pub fn with_style(mut self, style: Style) -> Self {
        *self.base.style = style;
        self
    }
}

impl Display for TextComponent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for component in FormattedText::Text(self.clone()).into_iter() {
            let component_text = match &component {
                FormattedText::Text(c) => c.text.to_string(),
                FormattedText::Translatable(c) => c.read()?.to_string(),
            };

            f.write_str(&component_text)?;
        }

        Ok(())
    }
}


