#[cfg(all(feature = "azalea-buf", feature = "simdnbt"))]
use std::io::{self, Cursor, Write};
use std::{
    fmt::{self, Display},
    sync::LazyLock,
};

#[cfg(all(feature = "azalea-buf", feature = "simdnbt"))]
use azalea_buf::{AzBuf, BufReadError};
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::{
    base_component::BaseComponent,
    style::{ChatFormatting, Style},
    text_component::TextComponent,
    translatable_component::{PrimitiveOrComponent, TranslatableComponent},
};

#[cfg(feature = "simdnbt")]
use crate::base_component::RawComponentInteractionProbe;



#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum FormattedText {
    Text(TextComponent),
    Translatable(TranslatableComponent),
}

pub static DEFAULT_STYLE: LazyLock<Style> = LazyLock::new(|| Style {
    color: Some(ChatFormatting::White.try_into().unwrap()),
    ..Style::default()
});

impl FormattedText {
    pub fn get_base_mut(&mut self) -> &mut BaseComponent {
        match self {
            Self::Text(c) => &mut c.base,
            Self::Translatable(c) => &mut c.base,
        }
    }

    pub fn get_base(&self) -> &BaseComponent {
        match self {
            Self::Text(c) => &c.base,
            Self::Translatable(c) => &c.base,
        }
    }

    fn append(&mut self, sibling: FormattedText) {
        self.get_base_mut().siblings.push(sibling);
    }

    fn parse_separator(
        json: &serde_json::Value,
    ) -> Result<Option<FormattedText>, serde_json::Error> {
        if let Some(separator) = json.get("separator") {
            return Ok(Some(FormattedText::deserialize(separator)?));
        }
        Ok(None)
    }

    #[cfg(feature = "simdnbt")]
    fn parse_separator_nbt(nbt: &simdnbt::borrow::NbtCompound) -> Option<FormattedText> {
        use simdnbt::FromNbtTag;

        if let Some(separator) = nbt.get("separator") {
            FormattedText::from_nbt_tag(separator)
        } else {
            None
        }
    }

    pub fn to_custom_format<F, S, C>(
        &self,
        mut style_formatter: F,
        mut text_formatter: S,
        mut cleanup_formatter: C,
        default_style: &Style,
    ) -> String
    where
        F: FnMut(&Style, &Style) -> (String, String),
        S: FnMut(&str) -> String,
        C: FnMut(&Style) -> String,
    {
        let mut output = String::new();

        let mut running_style = Style::default();
        self.to_custom_format_recursive(
            &mut output,
            &mut style_formatter,
            &mut text_formatter,
            &default_style.clone(),
            &mut running_style,
        );
        output.push_str(&cleanup_formatter(&running_style));

        output
    }

    fn to_custom_format_recursive<F, S>(
        &self,
        output: &mut String,
        style_formatter: &mut F,
        text_formatter: &mut S,
        parent_style: &Style,
        running_style: &mut Style,
    ) where
        F: FnMut(&Style, &Style) -> (String, String),
        S: FnMut(&str) -> String,
    {
        let component_style = &self.get_base().style;
        let new_style = parent_style.merged_with(component_style);

        let mut append_text = |text: &str| {
            if !text.is_empty() {
                let (formatted_style_prefix, formatted_style_suffix) =
                    style_formatter(running_style, &new_style);
                let formatted_text = text_formatter(text);

                output.push_str(&formatted_style_prefix);
                output.push_str(&formatted_text);
                output.push_str(&formatted_style_suffix);

                *running_style = new_style.clone();
            }
        };

        match &self {
            Self::Text(c) => {
                append_text(&c.text);
            }
            Self::Translatable(c) => match c.read() {
                Ok(c) => {
                    FormattedText::Text(c).to_custom_format_recursive(
                        output,
                        style_formatter,
                        text_formatter,
                        &new_style,
                        running_style,
                    );
                }
                Err(_) => {
                    append_text(&c.key);
                }
            },
        };

        for sibling in &self.get_base().siblings {
            sibling.to_custom_format_recursive(
                output,
                style_formatter,
                text_formatter,
                &new_style,
                running_style,
            );
        }
    }

    pub fn to_ansi_with_custom_style(&self, default_style: &Style) -> String {
        self.to_custom_format(
            |running, new| (running.compare_ansi(new), "".to_owned()),
            |text| text.to_owned(),
            |style| if !style.is_empty() { "\u{1b}[m" } else { "" }.to_owned(),
            default_style,
        )
    }

    pub fn to_ansi(&self) -> String {
        self.to_ansi_with_custom_style(&DEFAULT_STYLE)
    }

    pub fn to_html(&self) -> String {
        self.to_custom_format(
            |running, new| {
                (
                    format!(
                        "<span style=\"{}\">",
                        running.merged_with(new).get_html_style()
                    ),
                    "</span>".to_owned(),
                )
            },
            |text| {
                text.replace("&", "&amp;")
                    .replace("<", "&lt;")
                    .replace(">", "&gt;")
                    .replace("\n", "<br>")
            },
            |_| "".to_owned(),
            &DEFAULT_STYLE,
        )
    }
}

impl IntoIterator for FormattedText {
    type Item = FormattedText;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let base = self.get_base();
        let siblings = base.siblings.clone();
        let mut v: Vec<FormattedText> = Vec::with_capacity(siblings.len() + 1);
        v.push(self);
        for sibling in siblings {
            v.extend(sibling);
        }

        v.into_iter()
    }
}

impl<'de> Deserialize<'de> for FormattedText {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let json: serde_json::Value = serde::Deserialize::deserialize(de)?;

        let mut component: FormattedText;

        if !json.is_array() && !json.is_object() {
            return Ok(FormattedText::Text(TextComponent::new(
                json.as_str().unwrap_or("").to_owned(),
            )));
        }
        else if json.is_object() {
            if let Some(text) = json.get("text") {
                let text = text.as_str().unwrap_or("").to_owned();
                component = FormattedText::Text(TextComponent::new(text));
            } else if let Some(translate) = json.get("translate") {
                let translate = translate
                    .as_str()
                    .ok_or_else(|| de::Error::custom("\"translate\" must be a string"))?
                    .into();
                let fallback = if let Some(fallback) = json.get("fallback") {
                    Some(
                        fallback
                            .as_str()
                            .ok_or_else(|| de::Error::custom("\"fallback\" must be a string"))?
                            .to_owned(),
                    )
                } else {
                    None
                };
                if let Some(with) = json.get("with") {
                    let with_array = with
                        .as_array()
                        .ok_or_else(|| de::Error::custom("\"with\" must be an array"))?
                        .iter()
                        .map(|item| {
                            PrimitiveOrComponent::deserialize(item).map_err(de::Error::custom)
                        })
                        .collect::<Result<Vec<PrimitiveOrComponent>, _>>()?;

                    component = FormattedText::Translatable(TranslatableComponent::with_fallback(
                        translate, fallback, with_array,
                    ));
                } else {
                    component = FormattedText::Translatable(TranslatableComponent::with_fallback(
                        translate,
                        fallback,
                        Vec::new(),
                    ));
                }
            } else if let Some(score) = json.get("score") {
                if score.get("name").is_none() || score.get("objective").is_none() {
                    return Err(de::Error::missing_field(
                        "A score component needs at least a name and an objective",
                    ));
                }
                return Err(de::Error::custom(
                    "score text components aren't yet supported",
                ));
            } else if json.get("selector").is_some() {
                return Err(de::Error::custom(
                    "selector text components aren't yet supported",
                ));
            } else if json.get("keybind").is_some() {
                return Err(de::Error::custom(
                    "keybind text components aren't yet supported",
                ));
            } else if json.get("object").is_some() {
                return Err(de::Error::custom(
                    "object text components aren't yet supported",
                ));
            } else {
                let Some(_nbt) = json.get("nbt") else {
                    return Err(de::Error::custom(
                        format!("Don't know how to turn {json} into a FormattedText").as_str(),
                    ));
                };
                let _separator =
                    FormattedText::parse_separator(&json).map_err(de::Error::custom)?;

                let _interpret = match json.get("interpret") {
                    Some(v) => v.as_bool().ok_or(Some(false)).unwrap(),
                    None => false,
                };
                if let Some(_block) = json.get("block") {}
                return Err(de::Error::custom(
                    "nbt text components aren't yet supported",
                ));
            }
            if let Some(extra) = json.get("extra") {
                let Some(extra) = extra.as_array() else {
                    return Err(de::Error::custom("Extra isn't an array"));
                };
                if extra.is_empty() {
                    return Err(de::Error::custom("Unexpected empty array of components"));
                }
                for extra_component in extra {
                    let sibling =
                        FormattedText::deserialize(extra_component).map_err(de::Error::custom)?;
                    component.append(sibling);
                }
            }

            let style = Style::deserialize(&json);
            *component.get_base_mut().style = style;

            return Ok(component);
        }
        else if !json.is_array() {
            return Err(de::Error::custom(
                format!("Don't know how to turn {json} into a FormattedText").as_str(),
            ));
        }
        let json_array = json.as_array().unwrap();
        if json_array.is_empty() {
            return Ok(FormattedText::default());
        }
        let mut component =
            FormattedText::deserialize(&json_array[0]).map_err(de::Error::custom)?;
        for i in 1..json_array.len() {
            component.append(
                FormattedText::deserialize(json_array.get(i).unwrap())
                    .map_err(de::Error::custom)?,
            );
        }
        Ok(component)
    }
}

#[cfg(feature = "simdnbt")]
impl simdnbt::Serialize for FormattedText {
    fn to_compound(self) -> simdnbt::owned::NbtCompound {
        match self {
            FormattedText::Text(c) => c.to_compound(),
            FormattedText::Translatable(c) => c.to_compound(),
        }
    }
}

#[cfg(feature = "simdnbt")]
impl simdnbt::FromNbtTag for FormattedText {
    fn from_nbt_tag(tag: simdnbt::borrow::NbtTag) -> Option<Self> {
        let raw_interaction_probe = RawComponentInteractionProbe::from_protocol_component_tag(tag);

        let mut component = if let Some(string) = tag.string() {
            Some(FormattedText::from_nbt_string(string))
        }
        else if let Some(compound) = tag.compound() {
            FormattedText::from_nbt_compound(compound)
        }
        else if let Some(list) = tag.list() {
            FormattedText::from_nbt_list(list)
        } else {
            None
        };

        if let Some(component) = component.as_mut() {
            component.get_base_mut().raw_interaction_probe = Some(raw_interaction_probe);
        }

        component
    }
}

#[cfg(feature = "simdnbt")]
impl FormattedText {
    fn from_nbt_string(s: &simdnbt::Mutf8Str) -> Self {
        FormattedText::from(s)
    }
    fn from_nbt_list(list: simdnbt::borrow::NbtList) -> Option<FormattedText> {
        let mut components = Vec::new();
        if let Some(compounds) = list.compounds() {
            for compound in compounds {
                components.push(Self::from_nbt_list_compound(compound)?);
            }
        } else if let Some(strings) = list.strings() {
            for &string in strings {
                components.push(Self::from_nbt_string(string));
            }
        } else if let Some(lists) = list.lists() {
            for child in lists {
                components.push(Self::from_nbt_list(child)?);
            }
        } else {
            return None;
        }
        let mut iter = components.into_iter();
        let mut component = iter.next()?;
        for child in iter {
            component.append(child);
        }
        Some(component)
    }

    fn from_nbt_list_compound(compound: simdnbt::borrow::NbtCompound) -> Option<Self> {
        use simdnbt::FromNbtTag;
        if compound.len() == 1
            && let Some(tag) = compound.get("")
        {
            return Self::from_nbt_tag(tag);
        }
        Self::from_nbt_compound(compound)
    }

    fn translation_nbt_args(list: simdnbt::borrow::NbtList) -> Option<Vec<PrimitiveOrComponent>> {
        use PrimitiveOrComponent as P;
        use simdnbt::FromNbtTag;
        if list.empty() {
            return Some(Vec::new());
        }
        if let Some(values) = list.bytes() {
            return Some(values.iter().map(|&v| P::Short(i16::from(v))).collect());
        }
        if let Some(values) = list.shorts() {
            return Some(values.into_iter().map(P::Short).collect());
        }
        if let Some(values) = list.ints() {
            return Some(values.into_iter().map(P::Integer).collect());
        }
        if let Some(values) = list.longs() {
            return Some(values.into_iter().map(P::Long).collect());
        }
        if let Some(values) = list.floats() {
            return Some(values.into_iter().map(P::Float).collect());
        }
        if let Some(values) = list.doubles() {
            return Some(values.into_iter().map(P::Double).collect());
        }
        if let Some(values) = list.strings() {
            return Some(values.iter().map(|v| P::String(v.to_string())).collect());
        }
        if let Some(values) = list.lists() {
            return values
                .into_iter()
                .map(|v| Self::from_nbt_list(v).map(P::FormattedText))
                .collect();
        }
        if let Some(values) = list.compounds() {
            return values
                .into_iter()
                .map(|compound| {
                    if compound.len() == 1
                        && let Some(tag) = compound.get("")
                    {
                        if let Some(v) = tag.byte() {
                            return Some(P::Short(i16::from(v)));
                        }
                        if let Some(v) = tag.short() {
                            return Some(P::Short(v));
                        }
                        if let Some(v) = tag.int() {
                            return Some(P::Integer(v));
                        }
                        if let Some(v) = tag.long() {
                            return Some(P::Long(v));
                        }
                        if let Some(v) = tag.float() {
                            return Some(P::Float(v));
                        }
                        if let Some(v) = tag.double() {
                            return Some(P::Double(v));
                        }
                        if let Some(v) = tag.string() {
                            return Some(P::String(v.to_string()));
                        }
                        return Some(
                            Self::from_nbt_tag(tag)
                                .map(P::FormattedText)
                                .unwrap_or_else(|| P::String("?".into())),
                        );
                    }
                    Some(
                        Self::from_nbt_compound(compound)
                            .map(P::FormattedText)
                            .unwrap_or_else(|| P::String("?".into())),
                    )
                })
                .collect();
        }
        None
    }

    fn nbt_shape(tag: simdnbt::borrow::NbtTag, depth: usize) -> String {
        if depth == 0 {
            return "...".into();
        }
        if let Some(compound) = tag.compound() {
            let mut fields = Vec::new();
            for key in [
                "text",
                "translate",
                "score",
                "selector",
                "keybind",
                "object",
                "nbt",
                "",
                "extra",
                "with",
            ] {
                if let Some(value) = compound.get(key) {
                    fields.push(format!(
                        "{}:{}",
                        if key.is_empty() { "wrapper" } else { key },
                        Self::nbt_shape(value, depth - 1)
                    ));
                }
            }
            format!("compound({})", fields.join(","))
        } else if let Some(list) = tag.list() {
            let child = if let Some(compounds) = list.compounds() {
                compounds
                    .into_iter()
                    .take(2)
                    .map(|compound| {
                        let keys = [
                            "text",
                            "translate",
                            "score",
                            "selector",
                            "keybind",
                            "object",
                            "nbt",
                            "",
                            "extra",
                            "with",
                        ];
                        let fields = keys
                            .into_iter()
                            .filter_map(|key| {
                                compound.get(key).map(|value| {
                                    format!(
                                        "{}:{}",
                                        if key.is_empty() { "wrapper" } else { key },
                                        Self::nbt_shape(value, depth - 1)
                                    )
                                })
                            })
                            .collect::<Vec<_>>();
                        format!("compound({})", fields.join(","))
                    })
                    .collect::<Vec<_>>()
                    .join(",")
            } else {
                String::new()
            };
            format!("list<{}>({child})", list.id())
        } else {
            format!("tag{}", tag.id())
        }
    }

    pub fn from_nbt_compound(compound: simdnbt::borrow::NbtCompound) -> Option<Self> {
        use simdnbt::Deserialize;
        use tracing::trace;

        let mut component: FormattedText;

        if let Some(text) = compound.get("text") {
            let text = text.string().unwrap_or_default().to_string();
            component = FormattedText::Text(TextComponent::new(text));
        } else if let Some(translate) = compound.get("translate") {
            let translate = translate.string()?.into();
            let args = match compound.get("with") {
                Some(with) => Self::translation_nbt_args(with.list()?)?,
                None => Vec::new(),
            };
            let fallback = compound.string("fallback").map(|v| v.to_string());
            component = FormattedText::Translatable(TranslatableComponent::with_fallback(
                translate, fallback, args,
            ));
        } else if let Some(score) = compound.compound("score") {
            if score.get("name").is_none() || score.get("objective").is_none() {
                trace!("A score component needs at least a name and an objective");
                return None;
            }
            return None;
        } else if compound.get("selector").is_some() {
            trace!("selector text components aren't supported");
            return None;
        } else if compound.get("keybind").is_some() {
            trace!("keybind text components aren't supported");
            return None;
        } else if compound.get("object").is_some() {
            trace!("object text components aren't supported");
            return None;
        } else {
            let _nbt = compound.get("nbt")?;
            let _separator = FormattedText::parse_separator_nbt(&compound)?;

            let _interpret = match compound.get("interpret") {
                Some(v) => v.byte().unwrap_or_default() != 0,
                None => false,
            };
            if let Some(_block) = compound.get("block") {}
            trace!("nbt text components aren't yet supported");
            return None;
        }
        if let Some(extra) = compound.get("extra") {
            if let Some(items) = extra.list() {
                if let Some(items) = items.compounds() {
                    if items.is_empty() {
                        return None;
                    }
                    for item in items {
                        component.append(FormattedText::from_nbt_list_compound(item)?);
                    }
                } else if let Some(items) = items.strings() {
                    if items.is_empty() {
                        return None;
                    }
                    for item in items {
                        component.append(FormattedText::from_nbt_string(item));
                    }
                } else if let Some(items) = items.lists() {
                    if items.is_empty() {
                        return None;
                    }
                    for item in items {
                        component.append(FormattedText::from_nbt_list(item)?);
                    }
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }

        let base_style = Style::from_compound(compound).ok()?;
        let new_style = &mut component.get_base_mut().style;
        **new_style = new_style.merged_with(&base_style);

        Some(component)
    }
}

#[cfg(feature = "simdnbt")]
impl RawComponentInteractionProbe {
    fn from_protocol_component_tag(tag: simdnbt::borrow::NbtTag) -> Self {
        let mut probe = Self::default();
        probe.inspect_component_tag(tag);
        probe
    }

    fn inspect_component_tag(&mut self, tag: simdnbt::borrow::NbtTag) {
        if tag.string().is_some() {
            self.component_nodes += 1;
        } else if let Some(compound) = tag.compound() {
            self.inspect_component_compound(compound);
        } else if let Some(list) = tag.list() {
            self.inspect_component_list(list);
        } else {
            self.component_nodes += 1;
        }
    }

    fn inspect_component_list(&mut self, list: simdnbt::borrow::NbtList) {
        if let Some(compounds) = list.compounds() {
            for compound in compounds {
                self.inspect_component_compound(compound);
            }
        } else if let Some(strings) = list.strings() {
            self.component_nodes += strings.len();
        } else if let Some(lists) = list.lists() {
            for list in lists {
                self.inspect_component_list(list);
            }
        }
    }

    fn inspect_component_compound(&mut self, compound: simdnbt::borrow::NbtCompound) {
        self.component_nodes += 1;

        self.canonical_click_event_fields += usize::from(compound.contains("click_event"));
        self.legacy_click_event_fields += usize::from(compound.contains("clickEvent"));
        self.insertion_fields += usize::from(compound.contains("insertion"));

        for key in ["extra", "with", "separator"] {
            if let Some(child) = compound.get(key) {
                self.inspect_component_tag(child);
            }
        }
    }
}

#[cfg(feature = "simdnbt")]
impl From<&simdnbt::Mutf8Str> for FormattedText {
    fn from(s: &simdnbt::Mutf8Str) -> Self {
        FormattedText::Text(TextComponent::new(s.to_string()))
    }
}

#[cfg(all(feature = "azalea-buf", feature = "simdnbt"))]
impl AzBuf for FormattedText {
    fn azalea_read(buf: &mut Cursor<&[u8]>) -> Result<Self, BufReadError> {
        use simdnbt::FromNbtTag;
        use tracing::trace;

        let nbt = simdnbt::borrow::read_optional_tag(buf)?;
        trace!(
            "Reading NBT for FormattedText: {:?}",
            nbt.as_ref().map(|n| n.as_tag().to_owned())
        );
        match nbt {
            Some(nbt) => FormattedText::from_nbt_tag(nbt.as_tag()).ok_or_else(|| {
                let mut shape = Self::nbt_shape(nbt.as_tag(), 3);
                shape.truncate(shape.len().min(512));                          
                BufReadError::Custom(format!(
                    "couldn't convert nbt to chat message: shape={shape}"
                ))
            }),
            _ => Ok(FormattedText::default()),
        }
    }
    fn azalea_write(&self, buf: &mut impl Write) -> io::Result<()> {
        use simdnbt::Serialize;

        let mut out = Vec::new();
        simdnbt::owned::BaseNbt::write_unnamed(&(self.clone().to_compound().into()), &mut out);
        buf.write_all(&out)
    }
}

impl From<String> for FormattedText {
    fn from(s: String) -> Self {
        FormattedText::Text(TextComponent {
            text: s,
            base: BaseComponent::default(),
        })
    }
}
impl From<&str> for FormattedText {
    fn from(s: &str) -> Self {
        Self::from(s.to_owned())
    }
}
impl From<TranslatableComponent> for FormattedText {
    fn from(c: TranslatableComponent) -> Self {
        FormattedText::Translatable(c)
    }
}
impl From<TextComponent> for FormattedText {
    fn from(c: TextComponent) -> Self {
        FormattedText::Text(c)
    }
}

impl Display for FormattedText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FormattedText::Text(c) => c.fmt(f),
            FormattedText::Translatable(c) => c.fmt(f),
        }
    }
}

impl Default for FormattedText {
    fn default() -> Self {
        FormattedText::Text(TextComponent::default())
    }
}


