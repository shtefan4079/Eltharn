use super::{DataComponent, ItemStack};
use el_nbt::Tag;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemEquipmentSlot {
    Mainhand,
    Offhand,
    Feet,
    Legs,
    Chest,
    Head,
    Body,
    Saddle,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemEquipmentSlotGroup {
    Any,
    Mainhand,
    Offhand,
    Hand,
    Feet,
    Legs,
    Chest,
    Head,
    Armor,
    Body,
    Saddle,
}
impl ItemEquipmentSlotGroup {
    pub const ALL: [Self; 11] = [
        Self::Any,
        Self::Mainhand,
        Self::Offhand,
        Self::Hand,
        Self::Feet,
        Self::Legs,
        Self::Chest,
        Self::Head,
        Self::Armor,
        Self::Body,
        Self::Saddle,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Any => "any",
            Self::Mainhand => "mainhand",
            Self::Offhand => "offhand",
            Self::Hand => "hand",
            Self::Feet => "feet",
            Self::Legs => "legs",
            Self::Chest => "chest",
            Self::Head => "head",
            Self::Armor => "armor",
            Self::Body => "body",
            Self::Saddle => "saddle",
        }
    }
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|group| group.name() == name)
    }
    pub fn test(self, slot: ItemEquipmentSlot) -> bool {
        use ItemEquipmentSlot as S;
        match self {
            Self::Any => true,
            Self::Hand => matches!(slot, S::Mainhand | S::Offhand),
            Self::Armor => {
                matches!(slot, S::Feet | S::Legs | S::Chest | S::Head | S::Body)
            }
            Self::Mainhand => slot == S::Mainhand,
            Self::Offhand => slot == S::Offhand,
            Self::Feet => slot == S::Feet,
            Self::Legs => slot == S::Legs,
            Self::Chest => slot == S::Chest,
            Self::Head => slot == S::Head,
            Self::Body => slot == S::Body,
            Self::Saddle => slot == S::Saddle,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAttributeOperation {
    AddValue,
    AddMultipliedBase,
    AddMultipliedTotal,
}
impl ItemAttributeOperation {
    pub fn id(self) -> u8 {
        match self {
            Self::AddValue => 0,
            Self::AddMultipliedBase => 1,
            Self::AddMultipliedTotal => 2,
        }
    }
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "add_value" => Self::AddValue,
            "add_multiplied_base" => Self::AddMultipliedBase,
            "add_multiplied_total" => Self::AddMultipliedTotal,
            _ => return None,
        })
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum ItemAttributeDisplay {
    Default,
    Hidden,
    Override(Tag),
}
#[derive(Debug, Clone, PartialEq)]
pub struct ItemAttributeEntry {
    pub attribute: String,
    pub id: String,
    pub amount: f64,
    pub operation: ItemAttributeOperation,
    pub slot: ItemEquipmentSlotGroup,
    pub display: ItemAttributeDisplay,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemAttributeModifiers {
    pub entries: Vec<ItemAttributeEntry>,
}
impl ItemAttributeModifiers {
    pub fn compute(&self, attribute: &str, base: f64, slot: ItemEquipmentSlot) -> f64 {
        let mut value = base;
        for entry in &self.entries {
            if entry.attribute == attribute && entry.slot.test(slot) {
                value += match entry.operation {
                    ItemAttributeOperation::AddValue => entry.amount,
                    ItemAttributeOperation::AddMultipliedBase => entry.amount * base,
                    ItemAttributeOperation::AddMultipliedTotal => entry.amount * value,
                };
            }
        }
        value
    }
    pub fn with_modifier_added(mut self, entry: ItemAttributeEntry) -> Self {
        self.entries
            .retain(|old| old.attribute != entry.attribute || old.id != entry.id);
        self.entries.push(entry);
        self
    }
    pub fn from_component(component: &DataComponent) -> Option<Self> {
        let DataComponent::Generic { value, .. } = component else {
            return None;
        };
        let values = match value {
            Tag::List(values) => values,
            Tag::Compound(fields) => match get(fields, "modifiers")? {
                Tag::List(values) => values,
                _ => return None,
            },
            _ => return None,
        };
        let mut entries = Vec::with_capacity(values.len());
        for value in values {
            let Tag::Compound(fields) = value else {
                return None;
            };
            let number = match get(fields, "amount")? {
                Tag::Double(v) => *v,
                Tag::Float(v) => f64::from(*v),
                Tag::Int(v) => f64::from(*v),
                Tag::Long(v) => *v as f64,
                _ => return None,
            };
            let slot = get(fields, "slot").map_or(Some(ItemEquipmentSlotGroup::Any), |v| {
                string(v).and_then(ItemEquipmentSlotGroup::parse)
            })?;
            let display = match get(fields, "display") {
                None => ItemAttributeDisplay::Default,
                Some(Tag::String(name)) => match name.as_str() {
                    "default" => ItemAttributeDisplay::Default,
                    "hidden" => ItemAttributeDisplay::Hidden,
                    _ => return None,
                },
                Some(Tag::Compound(display)) => {
                    if let Some(kind) = get(display, "type").and_then(string) {
                        match kind {
                            "default" => ItemAttributeDisplay::Default,
                            "hidden" => ItemAttributeDisplay::Hidden,
                            "override" => {
                                ItemAttributeDisplay::Override(get(display, "value")?.clone())
                            }
                            _ => return None,
                        }
                    } else if let Some(Tag::Compound(over)) = get(display, "override") {
                        ItemAttributeDisplay::Override(get(over, "text")?.clone())
                    } else {
                        return None;
                    }
                }
                _ => return None,
            };
            entries.push(ItemAttributeEntry {
                attribute: string(get(fields, "type")?)?.into(),
                id: string(get(fields, "id")?)?.into(),
                amount: number,
                operation: ItemAttributeOperation::parse(string(get(fields, "operation")?)?)?,
                slot,
                display,
            });
        }
        Some(Self { entries })
    }
}
fn get<'a>(fields: &'a [(String, Tag)], name: &str) -> Option<&'a Tag> {
    fields.iter().find(|(key, _)| key == name).map(|(_, v)| v)
}
fn string(value: &Tag) -> Option<&str> {
    if let Tag::String(value) = value {
        Some(value)
    } else {
        None
    }
}
impl ItemStack {
    pub fn attribute_modifiers(&self) -> ItemAttributeModifiers {
        self.components
            .get("minecraft:attribute_modifiers")
            .and_then(ItemAttributeModifiers::from_component)
            .unwrap_or_default()
    }
}
pub fn calculate_enchantment_level_value(value: &Tag, level: i32) -> Option<f32> {
    calculate_level_value(value, level, 0)
}
fn number(value: &Tag) -> Option<f32> {
    match value {
        Tag::Byte(v) => Some(*v as f32),
        Tag::Short(v) => Some(*v as f32),
        Tag::Int(v) => Some(*v as f32),
        Tag::Long(v) => Some(*v as f32),
        Tag::Float(v) => Some(*v),
        Tag::Double(v) => Some(*v as f32),
        _ => None,
    }
}
fn calculate_level_value(value: &Tag, level: i32, depth: usize) -> Option<f32> {
    if depth > 64 {
        return None;
    }
    if let Some(value) = number(value) {
        return Some(value);
    }
    let Tag::Compound(fields) = value else {
        return None;
    };
    let kind = string(get(fields, "type")?)?
        .strip_prefix("minecraft:")
        .unwrap_or(string(get(fields, "type")?)?);
    let nested = |name| calculate_level_value(get(fields, name)?, level, depth + 1);
    Some(match kind {
        "linear" => {
            number(get(fields, "base")?)?
                + number(get(fields, "per_level_above_first")?)? * level.wrapping_sub(1) as f32
        }
        "levels_squared" => level.wrapping_mul(level) as f32 + number(get(fields, "added")?)?,
        "clamped" => {
            let min = number(get(fields, "min")?)?;
            let max = number(get(fields, "max")?)?;
            if max <= min {
                return None;
            }
            nested("value")?.clamp(min, max)
        }
        "fraction" => {
            let denominator = nested("denominator")?;
            if denominator == 0.0 {
                0.0
            } else {
                nested("numerator")? / denominator
            }
        }
        "exponent" => f64::from(nested("base")?).powf(f64::from(nested("power")?)) as f32,
        "lookup" => {
            let Tag::List(values) = get(fields, "values")? else {
                return None;
            };
            if level <= 0 {
                return None;
            }
            if let Some(value) = values.get(level as usize - 1) {
                number(value)?
            } else {
                nested("fallback")?
            }
        }
        _ => return None,
    })
}
