use super::*;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct StackModelIdentity {
    count: i32,
    components: Vec<(String, ComponentIdentity)>,
}
impl StackModelIdentity {
    pub(super) fn new(stack: &ItemStack) -> Self {
        let mut components: Vec<_> = stack
            .components
            .merged
            .iter()
            .map(|(name, value)| (name.clone(), ComponentIdentity::from(value)))
            .collect();
        components.sort_by(|left, right| left.0.cmp(&right.0));
        Self {
            count: stack.count,
            components,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum ComponentIdentity {
    Damage(i32),
    MaxStackSize(i32),
    MaxDamage(i32),
    CustomName(String),
    Lore(Vec<String>),
    Food {
        nutrition: i32,
        saturation: u32,
        can_always_eat: bool,
    },
    Generic {
        type_name: String,
        value: TagIdentity,
    },
}
impl From<&DataComponent> for ComponentIdentity {
    fn from(component: &DataComponent) -> Self {
        match component {
            DataComponent::Damage(value) => Self::Damage(*value),
            DataComponent::MaxStackSize(value) => Self::MaxStackSize(*value),
            DataComponent::MaxDamage(value) => Self::MaxDamage(*value),
            DataComponent::CustomName(value) => Self::CustomName(value.clone()),
            DataComponent::Lore(value) => Self::Lore(value.clone()),
            DataComponent::Food(value) => Self::Food {
                nutrition: value.nutrition,
                saturation: value.saturation.to_bits(),
                can_always_eat: value.can_always_eat,
            },
            DataComponent::Generic { type_name, value } => Self::Generic {
                type_name: type_name.clone(),
                value: TagIdentity::from(value),
            },
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TagIdentity {
    End,
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(u32),
    Double(u64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<TagIdentity>),
    Compound(Vec<(String, TagIdentity)>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}
impl From<&Tag> for TagIdentity {
    fn from(tag: &Tag) -> Self {
        match tag {
            Tag::End => Self::End,
            Tag::Byte(value) => Self::Byte(*value),
            Tag::Short(value) => Self::Short(*value),
            Tag::Int(value) => Self::Int(*value),
            Tag::Long(value) => Self::Long(*value),
            Tag::Float(value) => Self::Float(value.to_bits()),
            Tag::Double(value) => Self::Double(value.to_bits()),
            Tag::ByteArray(value) => Self::ByteArray(value.clone()),
            Tag::String(value) => Self::String(value.clone()),
            Tag::List(value) => Self::List(value.iter().map(Self::from).collect()),
            Tag::Compound(value) => Self::Compound(
                value
                    .iter()
                    .map(|(name, tag)| (name.clone(), Self::from(tag)))
                    .collect(),
            ),
            Tag::IntArray(value) => Self::IntArray(value.clone()),
            Tag::LongArray(value) => Self::LongArray(value.clone()),
        }
    }
}
