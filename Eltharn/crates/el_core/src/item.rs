pub mod attributes;
pub use crate::item_stubs::BundleItem;
use crate::registry::BuiltInRegistries;
use crate::resource_location::ResourceLocation;
use el_nbt::Tag;
use std::collections::HashMap;
use std::sync::Arc;
pub fn is_vanilla_bundle_item_path(path: &str) -> bool {
    matches!(
        path,
        "bundle"
            | "white_bundle"
            | "orange_bundle"
            | "magenta_bundle"
            | "light_blue_bundle"
            | "yellow_bundle"
            | "lime_bundle"
            | "pink_bundle"
            | "gray_bundle"
            | "light_gray_bundle"
            | "cyan_bundle"
            | "purple_bundle"
            | "blue_bundle"
            | "brown_bundle"
            | "green_bundle"
            | "red_bundle"
            | "black_bundle"
    )
}
pub fn decorated_pot_pattern_asset(item_id: &str) -> Option<&'static str> {
    let (namespace, path) = item_id.split_once(':').unwrap_or(("minecraft", item_id));
    if namespace != "minecraft" {
        return None;
    }
    Some(match path {
        "angler_pottery_sherd" => "angler_pottery_pattern",
        "archer_pottery_sherd" => "archer_pottery_pattern",
        "arms_up_pottery_sherd" => "arms_up_pottery_pattern",
        "blade_pottery_sherd" => "blade_pottery_pattern",
        "brewer_pottery_sherd" => "brewer_pottery_pattern",
        "burn_pottery_sherd" => "burn_pottery_pattern",
        "danger_pottery_sherd" => "danger_pottery_pattern",
        "explorer_pottery_sherd" => "explorer_pottery_pattern",
        "flow_pottery_sherd" => "flow_pottery_pattern",
        "friend_pottery_sherd" => "friend_pottery_pattern",
        "guster_pottery_sherd" => "guster_pottery_pattern",
        "heart_pottery_sherd" => "heart_pottery_pattern",
        "heartbreak_pottery_sherd" => "heartbreak_pottery_pattern",
        "howl_pottery_sherd" => "howl_pottery_pattern",
        "miner_pottery_sherd" => "miner_pottery_pattern",
        "mourner_pottery_sherd" => "mourner_pottery_pattern",
        "plenty_pottery_sherd" => "plenty_pottery_pattern",
        "prize_pottery_sherd" => "prize_pottery_pattern",
        "scrape_pottery_sherd" => "scrape_pottery_pattern",
        "sheaf_pottery_sherd" => "sheaf_pottery_pattern",
        "shelter_pottery_sherd" => "shelter_pottery_pattern",
        "skull_pottery_sherd" => "skull_pottery_pattern",
        "snort_pottery_sherd" => "snort_pottery_pattern",
        _ => return None,
    })
}
fn compound_get<'a>(fields: &'a Vec<(String, Tag)>, key: &str) -> Option<&'a Tag> {
    fields.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}
#[derive(Clone, Debug, PartialEq)]
pub struct FoodProperties {
    pub nutrition: i32,
    pub saturation: f32,
    pub can_always_eat: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CrossbowChargeType {
    None,
    Arrow,
    Rocket,
}
impl CrossbowChargeType {
    pub const fn serialized_name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Arrow => "arrow",
            Self::Rocket => "rocket",
        }
    }
}
pub struct CrossbowItem;
impl CrossbowItem {
    pub const MAX_CHARGE_DURATION: f32 = 1.25;
    pub const DEFAULT_CHARGE_TICKS: u32 = 25;
    pub const DEFAULT_RANGE: i32 = 8;
    pub const START_SOUND_PERCENT: f32 = 0.2;
    pub const MID_SOUND_PERCENT: f32 = 0.5;
    pub const ARROW_POWER: f32 = 3.15;
    pub const FIREWORK_POWER: f32 = 1.6;
    pub const MOB_ARROW_POWER: f32 = 1.6;
    pub fn charge_duration_ticks(quick_charge_level: u32) -> u32 {
        let base_ticks = 25_i32;
        let reduction = (quick_charge_level as i32) * 5;
        (base_ticks - reduction).max(1) as u32
    }
    pub fn power_for_time(time_held_ticks: f32, charge_duration_ticks: f32) -> f32 {
        let duration = charge_duration_ticks.max(1.0);
        (time_held_ticks / duration).clamp(0.0, 1.0)
    }
    pub fn shooting_power(has_firework: bool) -> f32 {
        if has_firework {
            Self::FIREWORK_POWER
        } else {
            Self::ARROW_POWER
        }
    }
    pub fn shot_pitch(index: usize, random_float: f32) -> f32 {
        if index == 0 {
            1.0
        } else {
            let range_decider = if (index & 1) == 1 { 0.63 } else { 0.43 };
            1.0 / (random_float * 0.5 + 1.8) + range_decider
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ConsumableProperties {
    pub consume_seconds: f32,
    pub animation: String,
}
impl ConsumableProperties {
    pub const DEFAULT_CONSUME_SECONDS: f32 = 1.6;
    pub const DEFAULT_ANIMATION: &'static str = "eat";
    pub fn consume_ticks(&self) -> i32 {
        (self.consume_seconds * 20.0) as i32
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ItemSwingAnimationType {
    None,
    #[default]
    Whack,
    Stab,
}
impl ItemSwingAnimationType {
    fn from_serialized_name(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "whack" => Some(Self::Whack),
            "stab" => Some(Self::Stab),
            _ => None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemSwingAnimation {
    pub animation_type: ItemSwingAnimationType,
    pub duration: i32,
}
impl ItemSwingAnimation {
    pub const DEFAULT: Self = Self {
        animation_type: ItemSwingAnimationType::Whack,
        duration: 6,
    };
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticWeaponCondition {
    pub max_duration_ticks: i32,
    pub min_speed: f32,
    pub min_relative_speed: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KineticWeapon {
    pub contact_cooldown_ticks: i32,
    pub delay_ticks: i32,
    pub dismount_conditions: Option<KineticWeaponCondition>,
    pub knockback_conditions: Option<KineticWeaponCondition>,
    pub damage_conditions: Option<KineticWeaponCondition>,
    pub forward_movement: f32,
    pub damage_multiplier: f32,
}
#[derive(Clone, Debug, PartialEq)]
pub enum DataComponent {
    Damage(i32),
    MaxStackSize(i32),
    MaxDamage(i32),
    CustomName(String),
    Lore(Vec<String>),
    Food(FoodProperties),
    Generic { type_name: String, value: Tag },
}
#[derive(Clone, Debug, PartialEq)]
pub struct DataComponentPatch {
    pub map: HashMap<String, Option<DataComponent>>,
}
impl DataComponentPatch {
    pub fn empty() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    pub fn set(&mut self, key: &str, comp: DataComponent) {
        self.map.insert(key.to_string(), Some(comp));
    }
    pub fn remove(&mut self, key: &str) {
        self.map.insert(key.to_string(), None);
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PatchedDataComponentMap {
    pub prototype: Arc<HashMap<String, DataComponent>>,
    pub patch: DataComponentPatch,
    pub merged: HashMap<String, DataComponent>,
}
impl PatchedDataComponentMap {
    pub fn new(prototype: Arc<HashMap<String, DataComponent>>) -> Self {
        let merged = (*prototype).clone();
        Self {
            prototype,
            patch: DataComponentPatch::empty(),
            merged,
        }
    }
    pub fn get(&self, key: &str) -> Option<&DataComponent> {
        self.merged.get(key)
    }
    pub fn set(&mut self, key: &str, comp: DataComponent) {
        self.patch.set(key, comp.clone());
        self.merged.insert(key.to_string(), comp);
    }
    pub fn remove(&mut self, key: &str) {
        self.patch.remove(key);
        self.merged.remove(key);
    }
    pub fn is_empty(&self) -> bool {
        self.merged.is_empty()
    }
    pub fn has_patch(&self) -> bool {
        !self.patch.is_empty()
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub location: ResourceLocation,
    pub components: Arc<HashMap<String, DataComponent>>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ItemStack {
    pub item: Arc<Item>,
    pub count: i32,
    pub components: PatchedDataComponentMap,
}
impl ItemStack {
    pub fn new(item: Arc<Item>, count: i32) -> Self {
        let components = PatchedDataComponentMap::new(item.components.clone());
        ItemStack {
            item,
            count,
            components,
        }
    }
    pub fn empty() -> Self {
        let air = BuiltInRegistries::items()
            .get_by_location(&ResourceLocation::from_string("minecraft:air"))
            .clone();
        Self::new(air, 0)
    }
    pub fn is_empty(&self) -> bool {
        self.count <= 0 || self.item.location.path == "air"
    }
    pub fn is_vanilla_shield_item(&self) -> bool {
        self.item.location.namespace == "minecraft" && self.item.location.path == "shield"
    }
    pub fn is_vanilla_bow_item(&self) -> bool {
        self.item.location.namespace == "minecraft" && self.item.location.path == "bow"
    }
    pub fn food_properties(&self) -> Option<FoodProperties> {
        data_component_food(self.components.get("minecraft:food")?)
    }
    pub fn consumable_properties(&self) -> Option<ConsumableProperties> {
        data_component_consumable(self.components.get("minecraft:consumable")?)
    }
    pub fn swing_animation(&self) -> Option<ItemSwingAnimation> {
        match self.components.get("minecraft:swing_animation") {
            None => Some(ItemSwingAnimation::DEFAULT),
            Some(component) => data_component_swing_animation(component),
        }
    }
    pub fn kinetic_weapon(&self) -> Option<KineticWeapon> {
        self.components
            .get("minecraft:kinetic_weapon")
            .and_then(data_component_kinetic_weapon)
    }
    pub fn has_foil(&self) -> bool {
        if let Some(override_value) = self
            .components
            .get("minecraft:enchantment_glint_override")
            .and_then(data_component_bool)
        {
            return override_value;
        }
        if self.item.location.namespace == "minecraft" {
            match self.item.location.path.as_str() {
                "enchanted_golden_apple"
                | "experience_bottle"
                | "written_book"
                | "nether_star"
                | "enchanted_book"
                | "end_crystal"
                | "debug_stick" => {
                    return true;
                }
                "compass" if self.components.get("minecraft:lodestone_tracker").is_some() => {
                    return true;
                }
                _ => {}
            }
        }
        self.components
            .get("minecraft:enchantments")
            .is_some_and(data_component_has_enchantments)
    }
    pub fn is(&self, _tag: &crate::ItemTag) -> bool {
        false
    }
    pub fn split(&mut self, amount: i32) -> Self {
        let real_amount = amount.min(self.count);
        let result = self.copy_with_count(real_amount);
        self.count -= real_amount;
        result
    }
    pub fn copy(&self) -> Self {
        if self.is_empty() {
            return Self::empty();
        }
        Self {
            item: self.item.clone(),
            count: self.count,
            components: self.components.clone(),
        }
    }
    pub fn copy_and_clear(&mut self) -> Self {
        if self.is_empty() {
            return Self::empty();
        }
        let result = self.copy();
        self.count = 0;
        result
    }
    pub fn copy_with_count(&self, count: i32) -> Self {
        if self.is_empty() {
            return Self::empty();
        }
        let mut copy = self.copy();
        copy.count = count;
        copy
    }
    pub fn transmute_copy(&self, new_item: Arc<Item>, new_count: i32) -> Self {
        if self.is_empty() {
            return Self::empty();
        }
        Self {
            item: new_item,
            count: new_count,
            components: self.components.clone(),
        }
    }
    pub fn get_max_stack_size(&self) -> i32 {
        if let Some(DataComponent::MaxStackSize(m)) =
            self.components.get("minecraft:max_stack_size")
        {
            *m
        } else {
            64
        }
    }
    pub fn is_stackable(&self) -> bool {
        self.get_max_stack_size() > 1 && (!self.is_damageable_item() || !self.is_damaged())
    }
    pub fn is_damageable_item(&self) -> bool {
        self.get_max_damage() > 0 && !self.components.get("minecraft:unbreakable").is_some()
    }
    pub fn is_damaged(&self) -> bool {
        self.is_damageable_item() && self.get_damage_value() > 0
    }
    pub fn get_damage_value(&self) -> i32 {
        if let Some(DataComponent::Damage(d)) = self.components.get("minecraft:damage") {
            *d
        } else {
            0
        }
    }
    pub fn set_damage_value(&mut self, value: i32) {
        let max_damage = self.get_max_damage();
        self.components.set(
            "minecraft:damage",
            DataComponent::Damage(value.clamp(0, max_damage)),
        );
    }
    pub fn get_max_damage(&self) -> i32 {
        if let Some(DataComponent::MaxDamage(m)) = self.components.get("minecraft:max_damage") {
            *m
        } else {
            0
        }
    }
    pub fn is_broken(&self) -> bool {
        self.is_damageable_item() && self.get_damage_value() >= self.get_max_damage()
    }
    pub fn next_damage_will_break(&self) -> bool {
        self.is_damageable_item() && self.get_damage_value() >= self.get_max_damage() - 1
    }
    pub fn apply_damage(&mut self, new_damage: i32) {
        self.set_damage_value(new_damage);
        if self.is_broken() {
            self.count -= 1;
        }
    }
    pub fn hurt_and_break(&mut self, amount: i32) {
        if !self.is_damageable_item() {
            return;
        }
        let new_damage = self.get_damage_value() + amount;
        self.apply_damage(new_damage);
    }
    pub fn to_nbt(&self) -> Tag {
        let mut fields: Vec<(String, Tag)> = Vec::new();
        fields.push((
            "id".to_string(),
            Tag::String(self.item.location.to_string()),
        ));
        fields.push(("count".to_string(), Tag::Int(self.count)));
        if self.components.has_patch() {
            let mut patch_fields: Vec<(String, Tag)> = Vec::new();
            let mut sorted_keys: Vec<&String> = self.components.patch.map.keys().collect();
            sorted_keys.sort();
            for key in sorted_keys {
                let opt_comp = self.components.patch.map.get(key).unwrap();
                if let Some(comp) = opt_comp {
                    let tag = match comp {
                        DataComponent::Damage(d) => Tag::Int(*d),
                        DataComponent::MaxStackSize(m) => Tag::Int(*m),
                        DataComponent::MaxDamage(m) => Tag::Int(*m),
                        DataComponent::CustomName(name) => Tag::String(name.clone()),
                        DataComponent::Lore(lines) => {
                            Tag::List(lines.iter().map(|line| Tag::String(line.clone())).collect())
                        }
                        DataComponent::Food(food) => {
                            let food_fields = vec![
                                (
                                    "can_always_eat".to_string(),
                                    Tag::Byte(if food.can_always_eat { 1 } else { 0 }),
                                ),
                                ("nutrition".to_string(), Tag::Int(food.nutrition)),
                                ("saturation".to_string(), Tag::Float(food.saturation)),
                            ];
                            Tag::Compound(food_fields)
                        }
                        DataComponent::Generic { value, .. } => value.clone(),
                    };
                    patch_fields.push((key.clone(), tag));
                } else {
                    patch_fields.push((format!("!{}", key), Tag::Compound(vec![])));
                }
            }
            fields.push(("components".to_string(), Tag::Compound(patch_fields)));
        }
        fields.sort_by(|a, b| a.0.cmp(&b.0));
        Tag::Compound(fields)
    }
    fn warn_invalid_item_id(id: &str) {
        static WARNED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        WARNED.get_or_init(|| {
            eprintln!("el_core: ItemStack::from_nbt: invalid item id {id:?}; using an empty stack");
        });
    }
    pub fn from_nbt(tag: &Tag) -> Option<Self> {
        let fields = match tag {
            Tag::Compound(m) => m,
            _ => return None,
        };
        let id_tag = compound_get(&fields, "id")?;
        let id_str = match id_tag {
            Tag::String(s) => s,
            _ => return None,
        };
        let loc = match ResourceLocation::try_from_string(id_str) {
            Some(loc) => loc,
            None => {
                Self::warn_invalid_item_id(id_str);
                return Some(ItemStack::empty());
            }
        };
        let item = BuiltInRegistries::items().get_by_location(&loc).clone();
        let count = compound_get(&fields, "count")
            .and_then(tag_to_i32)
            .unwrap_or(1);
        let mut stack = ItemStack::new(item, count);
        if let Some(Tag::Compound(comp_fields)) = compound_get(&fields, "components") {
            for (key, val) in comp_fields {
                if key.starts_with("!") {
                    stack.components.remove(&key[1..]);
                    continue;
                }
                let comp = match key.as_str() {
                    "minecraft:damage" => {
                        let d = tag_to_i32(val).unwrap_or(0);
                        DataComponent::Damage(d)
                    }
                    "minecraft:max_stack_size" => {
                        let m = tag_to_i32(val).unwrap_or(64);
                        DataComponent::MaxStackSize(m)
                    }
                    "minecraft:max_damage" => {
                        let m = tag_to_i32(val).unwrap_or(0);
                        DataComponent::MaxDamage(m)
                    }
                    "minecraft:custom_name" => {
                        let s = component_tag_to_json_string(val).unwrap_or_default();
                        DataComponent::CustomName(s)
                    }
                    "minecraft:lore" => {
                        let mut lines = Vec::new();
                        if let Tag::List(list) = val {
                            for t in list {
                                if let Some(line) = component_tag_to_json_string(t) {
                                    lines.push(line);
                                }
                            }
                        }
                        DataComponent::Lore(lines)
                    }
                    "minecraft:food" => {
                        if let Tag::Compound(f_fields) = val {
                            let nutrition = compound_get(f_fields, "nutrition")
                                .and_then(tag_to_i32)
                                .unwrap_or(0);
                            let saturation = compound_get(f_fields, "saturation")
                                .and_then(tag_to_f32)
                                .unwrap_or(0.0);
                            let can_always_eat = compound_get(f_fields, "can_always_eat")
                                .map(|t| match t {
                                    Tag::Byte(b) => *b != 0,
                                    _ => false,
                                })
                                .unwrap_or(false);
                            DataComponent::Food(FoodProperties {
                                nutrition,
                                saturation,
                                can_always_eat,
                            })
                        } else {
                            DataComponent::Generic {
                                type_name: key.clone(),
                                value: val.clone(),
                            }
                        }
                    }
                    _ => DataComponent::Generic {
                        type_name: key.clone(),
                        value: val.clone(),
                    },
                };
                stack.components.set(key, comp);
            }
        }
        Some(stack)
    }
}
fn component_tag_to_json_string(tag: &Tag) -> Option<String> {
    fn to_json(tag: &Tag) -> Option<serde_json::Value> {
        Some(match tag {
            Tag::End => serde_json::Value::Null,
            Tag::Byte(value) => serde_json::Value::Bool(*value != 0),
            Tag::Short(value) => serde_json::Value::Number((*value as i64).into()),
            Tag::Int(value) => serde_json::Value::Number((*value as i64).into()),
            Tag::Long(value) => serde_json::Value::Number((*value).into()),
            Tag::Float(value) => {
                serde_json::Number::from_f64(*value as f64).map(serde_json::Value::Number)?
            }
            Tag::Double(value) => {
                serde_json::Number::from_f64(*value).map(serde_json::Value::Number)?
            }
            Tag::ByteArray(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(|value| serde_json::Value::Number((*value as i64).into()))
                    .collect(),
            ),
            Tag::String(value) => serde_json::Value::String(value.clone()),
            Tag::List(values) => {
                serde_json::Value::Array(values.iter().filter_map(to_json).collect())
            }
            Tag::Compound(fields) => serde_json::Value::Object(
                fields
                    .iter()
                    .filter_map(|(key, value)| Some((key.clone(), to_json(value)?)))
                    .collect(),
            ),
            Tag::IntArray(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(|value| serde_json::Value::Number((*value as i64).into()))
                    .collect(),
            ),
            Tag::LongArray(values) => serde_json::Value::Array(
                values
                    .iter()
                    .map(|value| serde_json::Value::Number((*value).into()))
                    .collect(),
            ),
        })
    }
    match tag {
        Tag::String(value) => Some(value.clone()),
        _ => serde_json::to_string(&to_json(tag)?).ok(),
    }
}
fn tag_to_i32(tag: &Tag) -> Option<i32> {
    match tag {
        Tag::Byte(b) => Some(*b as i32),
        Tag::Short(s) => Some(*s as i32),
        Tag::Int(i) => Some(*i),
        Tag::Long(l) => Some(*l as i32),
        _ => None,
    }
}
fn tag_to_bool(tag: &Tag) -> Option<bool> {
    match tag {
        Tag::Byte(value) => Some(*value != 0),
        Tag::Short(value) => Some(*value != 0),
        Tag::Int(value) => Some(*value != 0),
        Tag::Long(value) => Some(*value != 0),
        _ => None,
    }
}
fn data_component_food(component: &DataComponent) -> Option<FoodProperties> {
    match component {
        DataComponent::Food(food) => Some(food.clone()),
        DataComponent::Generic {
            value: Tag::Compound(fields),
            ..
        } => Some(FoodProperties {
            nutrition: compound_get(fields, "nutrition").and_then(tag_to_i32)?,
            saturation: compound_get(fields, "saturation").and_then(tag_to_f32)?,
            can_always_eat: compound_get(fields, "can_always_eat")
                .and_then(tag_to_bool)
                .unwrap_or(false),
        }),
        _ => None,
    }
}
fn data_component_consumable(component: &DataComponent) -> Option<ConsumableProperties> {
    let DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    } = component
    else {
        return None;
    };
    let consume_seconds = compound_get(fields, "consume_seconds").map_or(
        Some(ConsumableProperties::DEFAULT_CONSUME_SECONDS),
        tag_to_f32,
    )?;
    if !consume_seconds.is_finite() || consume_seconds < 0.0 {
        return None;
    }
    let animation = compound_get(fields, "animation").map_or_else(
        || Some(ConsumableProperties::DEFAULT_ANIMATION.to_string()),
        |tag| match tag {
            Tag::String(value) => Some(value.clone()),
            _ => None,
        },
    )?;
    Some(ConsumableProperties {
        consume_seconds,
        animation,
    })
}
fn data_component_swing_animation(component: &DataComponent) -> Option<ItemSwingAnimation> {
    let DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    } = component
    else {
        return None;
    };
    let animation_type = compound_get(fields, "type").map_or_else(
        || Some(ItemSwingAnimationType::Whack),
        |tag| match tag {
            Tag::String(value) => ItemSwingAnimationType::from_serialized_name(value),
            _ => None,
        },
    )?;
    let duration = compound_get(fields, "duration")
        .map_or(Some(ItemSwingAnimation::DEFAULT.duration), tag_to_i32)?;
    if duration <= 0 {
        return None;
    }
    Some(ItemSwingAnimation {
        animation_type,
        duration,
    })
}
fn data_component_kinetic_weapon(component: &DataComponent) -> Option<KineticWeapon> {
    let DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    } = component
    else {
        return None;
    };
    let contact_cooldown_ticks =
        compound_get(fields, "contact_cooldown_ticks").map_or(Some(10), tag_to_i32)?;
    let delay_ticks = compound_get(fields, "delay_ticks").map_or(Some(0), tag_to_i32)?;
    if contact_cooldown_ticks < 0 || delay_ticks < 0 {
        return None;
    }
    let dismount_conditions = match compound_get(fields, "dismount_conditions") {
        Some(tag) => Some(data_component_kinetic_weapon_condition(tag)?),
        None => None,
    };
    let knockback_conditions = match compound_get(fields, "knockback_conditions") {
        Some(tag) => Some(data_component_kinetic_weapon_condition(tag)?),
        None => None,
    };
    let damage_conditions = match compound_get(fields, "damage_conditions") {
        Some(tag) => Some(data_component_kinetic_weapon_condition(tag)?),
        None => None,
    };
    let forward_movement =
        compound_get(fields, "forward_movement").map_or(Some(0.0), tag_to_f32)?;
    let damage_multiplier =
        compound_get(fields, "damage_multiplier").map_or(Some(1.0), tag_to_f32)?;
    Some(KineticWeapon {
        contact_cooldown_ticks,
        delay_ticks,
        dismount_conditions,
        knockback_conditions,
        damage_conditions,
        forward_movement,
        damage_multiplier,
    })
}
fn data_component_kinetic_weapon_condition(tag: &Tag) -> Option<KineticWeaponCondition> {
    let Tag::Compound(fields) = tag else {
        return None;
    };
    let max_duration_ticks = compound_get(fields, "max_duration_ticks").and_then(tag_to_i32)?;
    if max_duration_ticks < 0 {
        return None;
    }
    let min_speed = compound_get(fields, "min_speed").map_or(Some(0.0), tag_to_f32)?;
    let min_relative_speed =
        compound_get(fields, "min_relative_speed").map_or(Some(0.0), tag_to_f32)?;
    Some(KineticWeaponCondition {
        max_duration_ticks,
        min_speed,
        min_relative_speed,
    })
}
fn data_component_bool(component: &DataComponent) -> Option<bool> {
    match component {
        DataComponent::Generic { value, .. } => match value {
            Tag::Byte(value) => Some(*value != 0),
            Tag::Short(value) => Some(*value != 0),
            Tag::Int(value) => Some(*value != 0),
            Tag::Long(value) => Some(*value != 0),
            _ => None,
        },
        _ => None,
    }
}
fn data_component_has_enchantments(component: &DataComponent) -> bool {
    let DataComponent::Generic { value, .. } = component else {
        return false;
    };
    match value {
        Tag::Compound(fields) => {
            if let Some(levels) = compound_get(fields, "levels") {
                return match levels {
                    Tag::Compound(levels) => !levels.is_empty(),
                    Tag::List(levels) => !levels.is_empty(),
                    _ => false,
                };
            }
            fields.iter().any(|(name, _)| name != "show_in_tooltip")
        }
        Tag::List(levels) => !levels.is_empty(),
        _ => false,
    }
}
fn tag_to_f32(tag: &Tag) -> Option<f32> {
    match tag {
        Tag::Float(f) => Some(*f),
        Tag::Double(d) => Some(*d as f32),
        _ => None,
    }
}
