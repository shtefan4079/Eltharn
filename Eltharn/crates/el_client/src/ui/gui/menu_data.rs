use el_core::{ResourceLocation, registry::BuiltInRegistries};
use el_gui::{
    AdvancementCriterionProgress, AdvancementDisplayData, AdvancementFrame, AdvancementNodeData,
    AdvancementProgressData, AdvancementsUpdate, GameRuleEditMetadata, GameRuleRow, GameRuleValue,
    StatisticsCell, StatisticsColumn, StatisticsData, StatisticsGroup, StatisticsRow,
};
use std::collections::HashMap;
pub(super) type StatKey = (String, String);
pub(super) fn statistics_data(values: &HashMap<StatKey, i32>) -> StatisticsData {
    let mut general = Vec::new();
    let mut custom_keys = crate::ui::i18n::keys_with_prefix("stat.minecraft.");
    custom_keys.sort_by(|left, right| {
        crate::ui::i18n::translate(left)
            .cmp(&crate::ui::i18n::translate(right))
            .then_with(|| left.cmp(right))
    });
    for key in custom_keys {
        let Some((namespace, path)) = key
            .strip_prefix("stat.")
            .and_then(|tail| tail.split_once('.'))
        else {
            continue;
        };
        let value = stat_value(values, "minecraft:custom", &format!("{namespace}:{path}"));
        general.push(StatisticsRow::new(
            key.clone(),
            crate::ui::i18n::translate(&key),
            vec![StatisticsCell::new(
                format_stat(path, value),
                Some(i64::from(value)),
            )],
        ));
    }
    let item_columns = [
        ("minecraft:mined", "stat_type.minecraft.mined"),
        ("minecraft:broken", "stat_type.minecraft.broken"),
        ("minecraft:crafted", "stat_type.minecraft.crafted"),
        ("minecraft:used", "stat_type.minecraft.used"),
        ("minecraft:picked_up", "stat_type.minecraft.picked_up"),
        ("minecraft:dropped", "stat_type.minecraft.dropped"),
    ];
    let columns = item_columns
        .iter()
        .map(|(key, label)| StatisticsColumn::new(*key, crate::ui::i18n::translate(label)))
        .collect::<Vec<_>>();
    let items = BuiltInRegistries::items();
    let blocks = BuiltInRegistries::blocks();
    let mut item_rows = Vec::new();
    for index in 0..items.size() {
        let item = items.get_by_id(index);
        let location = item.location.to_string();
        if item.location.namespace == "minecraft" && item.location.path == "air" {
            continue;
        }
        let mined = stat_value(values, "minecraft:mined", &location);
        let broken = stat_value(values, "minecraft:broken", &location);
        let crafted = stat_value(values, "minecraft:crafted", &location);
        let used = stat_value(values, "minecraft:used", &location);
        let picked_up = stat_value(values, "minecraft:picked_up", &location);
        let dropped = stat_value(values, "minecraft:dropped", &location);
        let mined = if mined > 0 {
            mined
        } else {
            let block_location = ResourceLocation::try_from_string(&location);
            block_location
                .filter(|loc| blocks.get_by_location(loc).location == *loc)
                .map(|_| stat_value(values, "minecraft:mined", &location))
                .unwrap_or(0)
        };
        if [mined, broken, crafted, used, picked_up, dropped]
            .iter()
            .all(|value| *value == 0)
        {
            continue;
        }
        let label = translated_registry_name("item", &location);
        item_rows.push(StatisticsRow::new(
            location,
            label,
            [mined, broken, crafted, used, picked_up, dropped]
                .into_iter()
                .map(|value| StatisticsCell::new(value.to_string(), Some(i64::from(value))))
                .collect(),
        ));
    }
    let mut seen = item_rows
        .iter()
        .map(|row| row.key.clone())
        .collect::<std::collections::HashSet<_>>();
    for ((category, block_id), value) in values {
        if category != "minecraft:mined" || *value <= 0 || seen.contains(block_id) {
            continue;
        }
        let Some(location) = ResourceLocation::try_from_string(block_id) else {
            continue;
        };
        if blocks.get_by_location(&location).location != location {
            continue;
        }
        seen.insert(block_id.clone());
        let mut cells = vec![StatisticsCell::new(
            value.to_string(),
            Some(i64::from(*value)),
        )];
        cells.extend(
            (0..item_columns.len() - 1).map(|_| StatisticsCell::new("-", None).muted(true)),
        );
        item_rows.push(StatisticsRow::new(
            block_id.clone(),
            translated_registry_name("block", block_id),
            cells,
        ));
    }
    item_rows.sort_by(|left, right| left.key.cmp(&right.key));
    let entities = BuiltInRegistries::entity_types();
    let mut mob_rows = Vec::new();
    for index in 0..entities.size() {
        let entity = entities.get_by_id(index);
        let entity_id = entity.location.to_string();
        let kills = stat_value(values, "minecraft:killed", &entity_id);
        let killed_by = stat_value(values, "minecraft:killed_by", &entity_id);
        if kills == 0 && killed_by == 0 {
            continue;
        }
        let name = translated_registry_name("entity", &entity_id);
        let kills_line = if kills == 0 {
            localized_format("stat_type.minecraft.killed.none", &[name.clone()])
        } else {
            localized_format(
                "stat_type.minecraft.killed",
                &[kills.to_string(), name.clone()],
            )
        };
        let killed_by_line = if killed_by == 0 {
            localized_format("stat_type.minecraft.killed_by.none", &[name.clone()])
        } else {
            localized_format(
                "stat_type.minecraft.killed_by",
                &[name.clone(), killed_by.to_string()],
            )
        };
        mob_rows.push(StatisticsRow::new(
            entity_id,
            name,
            vec![
                StatisticsCell::new(kills_line, Some(i64::from(kills))).muted(kills == 0),
                StatisticsCell::new(killed_by_line, Some(i64::from(killed_by)))
                    .muted(killed_by == 0),
            ],
        ));
    }
    StatisticsData {
        general: StatisticsGroup::new(Vec::new(), general),
        items: StatisticsGroup::new(columns, item_rows),
        mobs: StatisticsGroup::new(Vec::new(), mob_rows),
    }
}
pub(super) fn merge_awarded_stats(
    values: &mut HashMap<StatKey, i32>,
    packet: &el_protocol_backend::BackendAwardStats,
) {
    for stat in &packet.stats {
        values.insert(
            (stat.category.clone(), stat.value_id.clone()),
            stat.value.max(0),
        );
    }
}
pub(super) fn game_rule_rows(values: &HashMap<String, String>) -> Vec<GameRuleRow> {
    let mut rows = values
        .iter()
        .filter_map(|(key, serialized)| game_rule_row(key, serialized))
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.key.cmp(&right.key));
    rows
}
fn game_rule_row(key: &str, serialized: &str) -> Option<GameRuleRow> {
    let path = key.strip_prefix("minecraft:")?;
    let (value, edit) = match vanilla_game_rule_type(path)? {
        VanillaGameRuleType::Boolean => {
            let value = match serialized {
                "true" => true,
                "false" => false,
                _ => return None,
            };
            (GameRuleValue::Boolean(value), GameRuleEditMetadata::Boolean)
        }
        VanillaGameRuleType::Integer { minimum, maximum } => {
            let value = serialized.parse::<i32>().ok()?;
            (
                GameRuleValue::Integer(i64::from(value)),
                GameRuleEditMetadata::Integer {
                    minimum,
                    maximum,
                    step: 1,
                },
            )
        }
    };
    let translation_key = format!("gamerule.{key}");
    let translated = crate::ui::i18n::translate(&translation_key);
    let label = if translated == translation_key {
        humanize_game_rule_path(path)
    } else {
        translated.into_owned()
    };
    Some(GameRuleRow::new(key, label, value, Some(edit)))
}
#[derive(Clone, Copy)]
enum VanillaGameRuleType {
    Boolean,
    Integer { minimum: i64, maximum: i64 },
}
fn vanilla_game_rule_type(path: &str) -> Option<VanillaGameRuleType> {
    use VanillaGameRuleType::{Boolean as B, Integer as I};
    let kind = match path {
        "advance_time"
        | "advance_weather"
        | "allow_entering_nether_using_portals"
        | "block_drops"
        | "block_explosion_drop_decay"
        | "command_blocks_work"
        | "command_block_output"
        | "drowning_damage"
        | "elytra_movement_check"
        | "ender_pearls_vanish_on_death"
        | "entity_drops"
        | "fall_damage"
        | "fire_damage"
        | "forgive_dead_players"
        | "freeze_damage"
        | "global_sound_events"
        | "immediate_respawn"
        | "keep_inventory"
        | "lava_source_conversion"
        | "limited_crafting"
        | "locator_bar"
        | "log_admin_commands"
        | "mob_drops"
        | "mob_explosion_drop_decay"
        | "mob_griefing"
        | "natural_health_regeneration"
        | "player_movement_check"
        | "projectiles_can_break_blocks"
        | "pvp"
        | "raids"
        | "reduced_debug_info"
        | "send_command_feedback"
        | "show_advancement_messages"
        | "show_death_messages"
        | "spawner_blocks_work"
        | "spawn_mobs"
        | "spawn_monsters"
        | "spawn_patrols"
        | "spawn_phantoms"
        | "spawn_wandering_traders"
        | "spawn_wardens"
        | "spectators_generate_chunks"
        | "spread_vines"
        | "tnt_explodes"
        | "tnt_explosion_drop_decay"
        | "universal_anger"
        | "water_source_conversion" => B,
        "fire_spread_radius_around_player" => I {
            minimum: -1,
            maximum: i64::from(i32::MAX),
        },
        "max_block_modifications" => I {
            minimum: 1,
            maximum: i64::from(i32::MAX),
        },
        "max_command_forks"
        | "max_command_sequence_length"
        | "max_entity_cramming"
        | "players_nether_portal_creative_delay"
        | "players_nether_portal_default_delay"
        | "players_sleeping_percentage"
        | "random_tick_speed"
        | "respawn_radius" => I {
            minimum: 0,
            maximum: i64::from(i32::MAX),
        },
        "max_minecart_speed" => I {
            minimum: 1,
            maximum: 1000,
        },
        "max_snow_accumulation_height" => I {
            minimum: 0,
            maximum: 8,
        },
        _ => return None,
    };
    Some(kind)
}
fn humanize_game_rule_path(path: &str) -> String {
    path.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
pub(super) fn advancements_update(
    packet: &el_protocol_backend::BackendUpdateAdvancements,
) -> AdvancementsUpdate {
    let added_or_replaced = packet
        .added
        .iter()
        .map(|advancement| {
            let display = advancement.display.as_ref().map(|display| {
                let mut projected = AdvancementDisplayData::new(
                    crate::ui::gui::screen::container::flatten_text_component(&display.title),
                    crate::ui::gui::screen::container::flatten_text_component(&display.description),
                    display.icon.item_id.clone(),
                    match display.frame.as_str() {
                        "challenge" => AdvancementFrame::Challenge,
                        "goal" => AdvancementFrame::Goal,
                        _ => AdvancementFrame::Task,
                    },
                    display.x,
                    display.y,
                )
                .hidden(display.hidden)
                .icon_stack(display.icon.count, display.icon.components.clone());
                if let Some(background) = &display.background {
                    projected = projected.background_sprite(background.clone());
                }
                projected
            });
            AdvancementNodeData::new(
                advancement.id.clone(),
                advancement.parent_id.clone(),
                display,
                advancement.requirements.clone(),
            )
        })
        .collect();
    let progress_updates = packet
        .progress
        .iter()
        .map(|advancement| {
            AdvancementProgressData::new(
                advancement.id.clone(),
                advancement
                    .criteria
                    .iter()
                    .map(|criterion| {
                        AdvancementCriterionProgress::new(
                            criterion.criterion.clone(),
                            criterion
                                .acquired
                                .then(|| criterion.acquired_at_epoch_millis)
                                .flatten()
                                .and_then(|timestamp| i64::try_from(timestamp).ok()),
                        )
                    })
                    .collect(),
            )
        })
        .collect();
    AdvancementsUpdate {
        reset: packet.reset,
        removed_ids: packet.removed.clone(),
        added_or_replaced,
        progress_updates,
    }
}
fn stat_value(values: &HashMap<StatKey, i32>, category: &str, value_id: &str) -> i32 {
    values
        .get(&(category.to_owned(), value_id.to_owned()))
        .copied()
        .unwrap_or(0)
}
fn translated_registry_name(kind: &str, id: &str) -> String {
    let Some(location) = ResourceLocation::try_from_string(id) else {
        return id.to_owned();
    };
    let key = format!("{kind}.{}.{}", location.namespace, location.path);
    let translated = crate::ui::i18n::translate(&key);
    if translated == key {
        id.to_owned()
    } else {
        translated.into_owned()
    }
}
fn localized_format(key: &str, arguments: &[String]) -> String {
    let mut template = crate::ui::i18n::translate(key).into_owned();
    for argument in arguments {
        if let Some(position) = template.find("%s") {
            template.replace_range(position..position + 2, argument);
        }
    }
    template
}
fn format_stat(id: &str, value: i32) -> String {
    if id.ends_with("_one_cm") || id == "crouch_one_cm" {
        let cm = f64::from(value);
        let meters = cm / 100.0;
        let kilometers = meters / 1000.0;
        if kilometers > 0.5 {
            format!("{kilometers:.2} km")
        } else if meters > 0.5 {
            format!("{meters:.2} m")
        } else {
            format!("{value} cm")
        }
    } else if id.ends_with("_time") || id == "sneak_time" {
        let seconds = f64::from(value) / 20.0;
        let minutes = seconds / 60.0;
        let hours = minutes / 60.0;
        let days = hours / 24.0;
        let years = days / 365.0;
        if years > 0.5 {
            format!("{years:.2} y")
        } else if days > 0.5 {
            format!("{days:.2} d")
        } else if hours > 0.5 {
            format!("{hours:.2} h")
        } else if minutes > 0.5 {
            format!("{minutes:.2} min")
        } else {
            let seconds = if seconds.fract() == 0.0 {
                format!("{seconds:.1}")
            } else {
                seconds.to_string()
            };
            format!("{seconds} s")
        }
    } else if id.starts_with("damage_") {
        format!("{:.2}", f64::from(value) / 10.0)
    } else {
        grouped_integer(value)
    }
}
fn grouped_integer(value: i32) -> String {
    let digits = value.unsigned_abs().to_string();
    let mut output = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if value < 0 {
        output.push('-');
    }
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            output.push(',');
        }
        output.push(digit);
    }
    output
}
