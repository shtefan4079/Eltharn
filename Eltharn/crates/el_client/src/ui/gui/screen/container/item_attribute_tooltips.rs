use super::*;
use el_core::item::attributes::{
    ItemAttributeDisplay, ItemAttributeEntry, ItemAttributeOperation, ItemEquipmentSlotGroup,
    calculate_enchantment_level_value,
};
pub(super) fn add_item_attribute_tooltip_lines(
    item: &ItemStack,
    player_bases: Option<(f64, f64)>,
    lines: &mut Vec<TooltipLine>,
) {
    let modifiers = item.attribute_modifiers();
    let enchantment_modifiers = enchantment_attribute_entries(item);
    for group in ItemEquipmentSlotGroup::ALL {
        let mut first = true;
        for entry in modifiers.entries.iter().chain(&enchantment_modifiers) {
            if entry.slot != group || matches!(entry.display, ItemAttributeDisplay::Hidden) {
                continue;
            }
            if first {
                lines.push(TooltipLine::new(String::new(), TOOLTIP_TEXT));
                lines.push(TooltipLine::new(
                    crate::ui::i18n::translate(&format!("item.modifiers.{}", group.name())).into(),
                    [2.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0, 1.0],
                ));
                first = false;
            }
            match &entry.display {
                ItemAttributeDisplay::Hidden => {}
                ItemAttributeDisplay::Override(component) => {
                    if let Some((text, styled)) =
                        styled_text_from_tag_with_style(component, TextComponentStyle::default())
                    {
                        lines.push(TooltipLine::new(text, TOOLTIP_TEXT).with_styled_text(styled));
                    }
                }
                ItemAttributeDisplay::Default => {
                    if let Some(line) = default_modifier_line(entry, player_bases) {
                        lines.push(line);
                    }
                }
            }
        }
    }
}
fn enchantment_attribute_entries(item: &ItemStack) -> Vec<ItemAttributeEntry> {
    let Some(DataComponent::Generic {
        value: Tag::Compound(metadata),
        ..
    }) = item.components.get(ENCHANTMENT_TOOLTIP_METADATA_COMPONENT)
    else {
        return Vec::new();
    };
    let Some(Tag::Compound(definitions)) = compound_get(metadata, "definitions") else {
        return Vec::new();
    };
    let Some(DataComponent::Generic { value, .. }) = item.components.get("minecraft:enchantments")
    else {
        return Vec::new();
    };
    let mut levels = Vec::new();
    match value {
        Tag::Compound(fields) => {
            let fields = match compound_get(fields, "levels") {
                Some(Tag::Compound(levels)) => levels,
                _ => fields,
            };
            levels.extend(fields.iter().filter_map(|(id, value)| {
                tag_i32(value)
                    .filter(|&level| level > 0)
                    .map(|level| (id.as_str(), level))
            }));
        }
        Tag::List(entries) => {
            for entry in entries {
                let Tag::Compound(fields) = entry else {
                    continue;
                };
                let Some(Tag::String(id)) = compound_get(fields, "id") else {
                    continue;
                };
                let Some(level) = compound_get(fields, "lvl")
                    .or_else(|| compound_get(fields, "level"))
                    .and_then(tag_i32)
                    .filter(|&level| level > 0)
                else {
                    continue;
                };
                levels.push((id.as_str(), level));
            }
        }
        _ => {}
    }
    let mut result = Vec::new();
    for (enchantment, level) in levels {
        let Some(Tag::Compound(definition)) = compound_get(definitions, enchantment) else {
            continue;
        };
        let Some(Tag::List(slots)) = compound_get(definition, "slots") else {
            continue;
        };
        let Some(Tag::List(effects)) = compound_get(definition, "attribute_effects") else {
            continue;
        };
        for effect in effects {
            let Tag::Compound(effect) = effect else {
                continue;
            };
            let Some(Tag::String(attribute)) = compound_get(effect, "attribute") else {
                continue;
            };
            let Some(Tag::String(id)) = compound_get(effect, "id") else {
                continue;
            };
            let Some(amount) = compound_get(effect, "amount")
                .and_then(|amount| calculate_enchantment_level_value(amount, level))
            else {
                continue;
            };
            let operation = match compound_get(effect, "operation") {
                Some(Tag::String(operation)) => match operation.as_str() {
                    "add_value" => ItemAttributeOperation::AddValue,
                    "add_multiplied_base" => ItemAttributeOperation::AddMultipliedBase,
                    "add_multiplied_total" => ItemAttributeOperation::AddMultipliedTotal,
                    _ => continue,
                },
                _ => continue,
            };
            for slot in ItemEquipmentSlotGroup::ALL {
                if slots
                    .iter()
                    .any(|value| matches!(value, Tag::String(name) if name == slot.name()))
                {
                    result.push(ItemAttributeEntry {
                        attribute: attribute.clone(),
                        id: format!("{id}/{}", slot.name()),
                        amount: f64::from(amount),
                        operation,
                        slot,
                        display: ItemAttributeDisplay::Default,
                    });
                }
            }
        }
    }
    result
}
fn default_modifier_line(
    entry: &ItemAttributeEntry,
    player_bases: Option<(f64, f64)>,
) -> Option<TooltipLine> {
    default_modifier_line_with_translation(entry, player_bases, |key| {
        crate::ui::i18n::translate(key).into_owned()
    })
}
fn default_modifier_line_with_translation(
    entry: &ItemAttributeEntry,
    player_bases: Option<(f64, f64)>,
    translate: impl Fn(&str) -> String,
) -> Option<TooltipLine> {
    let mut amount = entry.amount;
    let mut display_with_base = false;
    if let Some((damage, speed)) = player_bases {
        match entry.id.as_str() {
            "minecraft:base_attack_damage" => {
                amount += damage;
                display_with_base = true;
            }
            "minecraft:base_attack_speed" => {
                amount += speed;
                display_with_base = true;
            }
            _ => {}
        }
    }
    let display_amount = match entry.operation {
        ItemAttributeOperation::AddMultipliedBase | ItemAttributeOperation::AddMultipliedTotal => {
            amount * 100.0
        }
        ItemAttributeOperation::AddValue if entry.attribute == "minecraft:knockback_resistance" => {
            amount * 10.0
        }
        ItemAttributeOperation::AddValue => amount,
    };
    let (kind, displayed, color) = if display_with_base {
        ("equals", display_amount, [0.0, 2.0 / 3.0, 0.0, 1.0])
    } else if amount > 0.0 {
        (
            "plus",
            display_amount,
            attribute_style(&entry.attribute, true),
        )
    } else if amount < 0.0 {
        (
            "take",
            -display_amount,
            attribute_style(&entry.attribute, false),
        )
    } else {
        return None;
    };
    let key = format!("attribute.modifier.{kind}.{}", entry.operation.id());
    let name = entry
        .attribute
        .strip_prefix("minecraft:")
        .map(|path| translate(&format!("attribute.name.{path}")))
        .unwrap_or_else(|| entry.attribute.clone());
    let mut text = format_two_percent_args(
        &translate(&key),
        &java_attribute_modifier_format(displayed),
        &name,
    );
    if display_with_base {
        text.insert(0, ' ');
    }
    Some(TooltipLine::new(text, color))
}
fn attribute_style(id: &str, increased: bool) -> [f32; 4] {
    let blue = [1.0 / 3.0, 1.0 / 3.0, 1.0, 1.0];
    let red = [1.0, 1.0 / 3.0, 1.0 / 3.0, 1.0];
    match id {
        "minecraft:gravity"
        | "minecraft:scale"
        | "minecraft:waypoint_transmit_range"
        | "minecraft:waypoint_receive_range" => [2.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0, 1.0],
        "minecraft:burning_time" | "minecraft:fall_damage_multiplier" => {
            if increased {
                red
            } else {
                blue
            }
        }
        _ => {
            if increased {
                blue
            } else {
                red
            }
        }
    }
}
