use crate::biome::Biome;
use crate::block::Block;
use crate::block_state::BlockState;
use crate::entity::EntityType;
use crate::item::{DataComponent, FoodProperties, Item, is_vanilla_bundle_item_path};
use crate::resource_location::ResourceLocation;
use el_nbt::Tag;
use std::any::Any;
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, OnceLock};
include!("generated/generated_registry_data.rs");
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResourceKey {
    pub registry: ResourceLocation,
    pub location: ResourceLocation,
}
impl ResourceKey {
    pub fn new(registry: ResourceLocation, location: ResourceLocation) -> Self {
        ResourceKey { registry, location }
    }
}
#[derive(Debug)]
pub struct ReferenceHolder<T> {
    key: ResourceKey,
    value: OnceLock<Arc<T>>,
}
impl<T> ReferenceHolder<T> {
    pub fn new(key: ResourceKey) -> Self {
        ReferenceHolder {
            key,
            value: OnceLock::new(),
        }
    }
    pub fn key(&self) -> &ResourceKey {
        &self.key
    }
    pub fn is_bound(&self) -> bool {
        self.value.get().is_some()
    }
    pub fn bind(&self, val: Arc<T>) -> Result<(), Arc<T>> {
        self.value.set(val)
    }
    pub fn get(&self) -> Option<&Arc<T>> {
        self.value.get()
    }
}
#[derive(Clone, Debug)]
pub enum Holder<T> {
    Direct(Arc<T>),
    Reference(Arc<ReferenceHolder<T>>),
}
impl<T> Holder<T> {
    pub fn value(&self) -> &T {
        match self {
            Holder::Direct(val) => val,
            Holder::Reference(ref_holder) => {
                ref_holder.get().expect("Reference holder is not bound yet")
            }
        }
    }
    pub fn value_arc(&self) -> Arc<T> {
        match self {
            Holder::Direct(val) => Arc::clone(val),
            Holder::Reference(ref_holder) => {
                Arc::clone(ref_holder.get().expect("Reference holder is not bound yet"))
            }
        }
    }
    pub fn key(&self) -> Option<&ResourceKey> {
        match self {
            Holder::Direct(_) => None,
            Holder::Reference(ref_holder) => Some(ref_holder.key()),
        }
    }
    pub fn is_bound(&self) -> bool {
        match self {
            Holder::Direct(_) => true,
            Holder::Reference(ref_holder) => ref_holder.is_bound(),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TagKey<T> {
    pub registry: ResourceLocation,
    pub location: ResourceLocation,
    _marker: std::marker::PhantomData<fn() -> T>,
}
impl<T> TagKey<T> {
    pub fn new(registry: ResourceLocation, location: ResourceLocation) -> Self {
        TagKey {
            registry,
            location,
            _marker: std::marker::PhantomData,
        }
    }
}
#[derive(Clone, Debug)]
pub enum HolderSet<T> {
    Direct(Vec<Holder<T>>),
    Named(TagKey<T>, Vec<Holder<T>>),
}
impl<T> HolderSet<T> {
    pub fn iter(&self) -> std::slice::Iter<'_, Holder<T>> {
        match self {
            HolderSet::Direct(vec) => vec.iter(),
            HolderSet::Named(_, vec) => vec.iter(),
        }
    }
}
pub struct Registry<T> {
    id_to_value: Vec<Arc<T>>,
    location_to_id: HashMap<ResourceLocation, usize>,
}
impl<T> Registry<T> {
    pub fn new() -> Self {
        Registry {
            id_to_value: Vec::new(),
            location_to_id: HashMap::new(),
        }
    }
    pub fn register(&mut self, location: ResourceLocation, value: T) -> usize {
        let id = self.id_to_value.len();
        self.id_to_value.push(Arc::new(value));
        self.location_to_id.insert(location, id);
        id
    }
    pub fn get_by_id(&self, id: usize) -> Option<&Arc<T>> {
        self.id_to_value.get(id)
    }
    pub fn get_by_location(&self, location: &ResourceLocation) -> Option<&Arc<T>> {
        self.location_to_id
            .get(location)
            .and_then(|id| self.id_to_value.get(*id))
    }
    pub fn get_id(&self, location: &ResourceLocation) -> Option<usize> {
        self.location_to_id.get(location).copied()
    }
}
pub struct DefaultedRegistry<T> {
    registry: Registry<T>,
    default_id: usize,
    default_location: ResourceLocation,
}
impl<T> DefaultedRegistry<T> {
    pub fn new(default_location: ResourceLocation) -> Self {
        DefaultedRegistry {
            registry: Registry::new(),
            default_id: 0,
            default_location,
        }
    }
    pub fn register(&mut self, location: ResourceLocation, value: T) -> usize {
        let is_default = location == self.default_location;
        let id = self.registry.register(location, value);
        if is_default {
            self.default_id = id;
        }
        id
    }
    pub fn get_by_id(&self, id: usize) -> &Arc<T> {
        self.registry.get_by_id(id).unwrap_or_else(|| {
            self.registry
                .get_by_id(self.default_id)
                .expect("default value must be registered")
        })
    }
    pub fn get_by_location(&self, location: &ResourceLocation) -> &Arc<T> {
        self.registry.get_by_location(location).unwrap_or_else(|| {
            self.registry
                .get_by_id(self.default_id)
                .expect("default value must be registered")
        })
    }
    pub fn get_id(&self, location: &ResourceLocation) -> usize {
        self.registry.get_id(location).unwrap_or(self.default_id)
    }
    pub fn default_id(&self) -> usize {
        self.default_id
    }
    pub fn default_location(&self) -> &ResourceLocation {
        &self.default_location
    }
    pub fn size(&self) -> usize {
        self.registry.id_to_value.len()
    }
}
pub struct MappedRegistry<T> {
    registry_key: ResourceLocation,
    id_to_value: Vec<Arc<T>>,
    location_to_id: HashMap<ResourceLocation, usize>,
    key_to_id: HashMap<ResourceKey, usize>,
    holders: HashMap<ResourceLocation, Arc<ReferenceHolder<T>>>,
    frozen: bool,
}
impl<T> MappedRegistry<T> {
    pub fn new(registry_key: ResourceLocation) -> Self {
        MappedRegistry {
            registry_key,
            id_to_value: Vec::new(),
            location_to_id: HashMap::new(),
            key_to_id: HashMap::new(),
            holders: HashMap::new(),
            frozen: false,
        }
    }
    pub fn registry_key(&self) -> &ResourceLocation {
        &self.registry_key
    }
    pub fn get_holder(&mut self, location: &ResourceLocation) -> Holder<T> {
        let key = ResourceKey::new(self.registry_key.clone(), location.clone());
        let ref_holder = self
            .holders
            .entry(location.clone())
            .or_insert_with(|| Arc::new(ReferenceHolder::new(key)));
        Holder::Reference(Arc::clone(ref_holder))
    }
    pub fn register(&mut self, location: ResourceLocation, value: T) -> usize {
        assert!(!self.frozen, "Registry is frozen");
        let id = self.id_to_value.len();
        let value_arc = Arc::new(value);
        self.id_to_value.push(Arc::clone(&value_arc));
        self.location_to_id.insert(location.clone(), id);
        let key = ResourceKey::new(self.registry_key.clone(), location.clone());
        self.key_to_id.insert(key.clone(), id);
        let ref_holder = self
            .holders
            .entry(location)
            .or_insert_with(|| Arc::new(ReferenceHolder::new(key)));
        let _ = ref_holder.bind(value_arc);
        id
    }
    pub fn freeze(&mut self) {
        self.frozen = true;
    }
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }
    pub fn get_by_id(&self, id: usize) -> Option<&Arc<T>> {
        self.id_to_value.get(id)
    }
    pub fn get_by_location(&self, location: &ResourceLocation) -> Option<&Arc<T>> {
        self.location_to_id
            .get(location)
            .and_then(|id| self.id_to_value.get(*id))
    }
    pub fn get_id(&self, location: &ResourceLocation) -> Option<usize> {
        self.location_to_id.get(location).copied()
    }
    pub fn size(&self) -> usize {
        self.id_to_value.len()
    }
}
pub struct RegistryAccess {
    registries: HashMap<ResourceLocation, Box<dyn Any + Send + Sync>>,
}
impl RegistryAccess {
    pub fn new() -> Self {
        RegistryAccess {
            registries: HashMap::new(),
        }
    }
    pub fn add_registry<T: 'static + Send + Sync>(
        &mut self,
        key: ResourceLocation,
        registry: MappedRegistry<T>,
    ) {
        self.registries.insert(key, Box::new(registry));
    }
    pub fn get_registry<T: 'static + Send + Sync>(
        &self,
        key: &ResourceLocation,
    ) -> Option<&MappedRegistry<T>> {
        self.registries
            .get(key)
            .and_then(|r| r.downcast_ref::<MappedRegistry<T>>())
    }
    pub fn get_registry_mut<T: 'static + Send + Sync>(
        &mut self,
        key: &ResourceLocation,
    ) -> Option<&mut MappedRegistry<T>> {
        self.registries
            .get_mut(key)
            .and_then(|r| r.downcast_mut::<MappedRegistry<T>>())
    }
}
pub struct BlockStateRegistry {
    states: Vec<BlockState>,
    lookup: HashMap<(ResourceLocation, Vec<(String, String)>), usize>,
    pub block_properties: HashMap<ResourceLocation, Vec<crate::block::Property>>,
}
impl BlockStateRegistry {
    pub fn load() -> Self {
        let mut raw_states = Vec::with_capacity(RAW_BLOCK_STATES.len());
        let mut prop_collector: HashMap<ResourceLocation, HashMap<String, BTreeSet<String>>> =
            HashMap::new();
        for (id, &(block_name_str, props)) in RAW_BLOCK_STATES.iter().enumerate() {
            let block_name = ResourceLocation::from_string(block_name_str);
            let mut properties = Vec::with_capacity(props.len());
            for &(key, value) in props {
                properties.push((key.to_string(), value.to_string()));
                prop_collector
                    .entry(block_name.clone())
                    .or_default()
                    .entry(key.to_string())
                    .or_default()
                    .insert(value.to_string());
            }
            raw_states.push((id, block_name, properties));
        }
        let mut block_properties = HashMap::new();
        for (block_name, collector) in prop_collector {
            let mut props = Vec::new();
            for (prop_name, values) in collector {
                let is_boolean =
                    (values.len() <= 2) && values.iter().all(|v| v == "true" || v == "false");
                let is_integer = values.iter().all(|v| v.parse::<i32>().is_ok());
                if is_boolean {
                    props.push(crate::block::Property::Boolean { name: prop_name });
                } else if is_integer {
                    let mut min_val = i32::MAX;
                    let mut max_val = i32::MIN;
                    for v in &values {
                        let parsed = v.parse::<i32>().unwrap();
                        min_val = min_val.min(parsed);
                        max_val = max_val.max(parsed);
                    }
                    props.push(crate::block::Property::Integer {
                        name: prop_name,
                        min: min_val,
                        max: max_val,
                    });
                } else {
                    let sorted_values: Vec<String> = values.into_iter().collect();
                    props.push(crate::block::Property::Enum {
                        name: prop_name,
                        values: sorted_values,
                    });
                }
            }
            props.sort_by(|a, b| a.name().cmp(b.name()));
            block_properties.insert(block_name, props);
        }
        let mut states = Vec::new();
        let mut lookup = HashMap::new();
        let max_id = raw_states.iter().map(|(id, _, _)| *id).max().unwrap_or(0);
        states.resize(
            max_id + 1,
            BlockState::new(Arc::new(ResourceLocation::from_string("minecraft:air"))),
        );
        for (id, block_name, properties) in raw_states {
            let block_loc_arc = Arc::new(block_name.clone());
            let mut state = BlockState::new(block_loc_arc.clone()).with_id(id);
            for (k, v) in &properties {
                state = state.with_property(k, v);
            }
            states[id] = state;
            let mut sorted_props = properties;
            sorted_props.sort_by(|a, b| a.0.cmp(&b.0));
            lookup.insert((block_name, sorted_props), id);
        }
        BlockStateRegistry {
            states,
            lookup,
            block_properties,
        }
    }
    pub fn get_state_by_id(&self, id: usize) -> Option<&BlockState> {
        self.states.get(id)
    }
    pub fn states(&self) -> &[BlockState] {
        &self.states
    }
    pub fn get_id_by_state(
        &self,
        block_name: &ResourceLocation,
        properties: &HashMap<String, String>,
    ) -> Option<usize> {
        let mut sorted_props: Vec<(String, String)> = properties
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        sorted_props.sort_by(|a, b| a.0.cmp(&b.0));
        self.lookup
            .get(&(block_name.clone(), sorted_props))
            .copied()
    }
}
pub static BLOCK_STATE_REGISTRY: OnceLock<BlockStateRegistry> = OnceLock::new();
pub static BLOCK_REGISTRY: OnceLock<DefaultedRegistry<Block>> = OnceLock::new();
pub static ITEM_REGISTRY: OnceLock<DefaultedRegistry<Item>> = OnceLock::new();
pub static ENTITY_TYPE_REGISTRY: OnceLock<DefaultedRegistry<EntityType>> = OnceLock::new();
pub static BIOME_REGISTRY: OnceLock<DefaultedRegistry<Biome>> = OnceLock::new();
pub fn bootstrap_blocks() -> DefaultedRegistry<Block> {
    let state_reg = BuiltInRegistries::block_states();
    let mut reg = DefaultedRegistry::new(ResourceLocation::from_string("minecraft:air"));
    for &(id, name, default_state_id) in RAW_BLOCKS {
        let loc = ResourceLocation::from_string(name);
        let properties = state_reg
            .block_properties
            .get(&loc)
            .cloned()
            .unwrap_or_default();
        let block = Block {
            location: loc.clone(),
            default_state_id,
            properties,
        };
        let registered_id = reg.register(loc, block);
        assert_eq!(
            registered_id, id,
            "Block ID mismatch during bootstrap for {name}"
        );
    }
    reg
}
pub fn bootstrap_items() -> DefaultedRegistry<Item> {
    let mut reg = DefaultedRegistry::new(ResourceLocation::from_string("minecraft:air"));
    for item_data in RAW_ITEMS {
        let id = item_data.id;
        let loc = ResourceLocation::from_string(item_data.name);
        let max_stack_size = item_data.max_stack_size;
        let max_damage = item_data.max_damage;
        let nutrition = item_data.nutrition;
        let saturation = item_data.saturation;
        let always_eat = item_data.always_eat;
        let food_properties = if let Some(nut) = nutrition {
            let sat = saturation.unwrap_or(0.0);
            Some(FoodProperties {
                nutrition: nut,
                saturation: sat,
                can_always_eat: always_eat,
            })
        } else {
            None
        };
        let mut components = HashMap::new();
        if max_stack_size != 64 {
            components.insert(
                "minecraft:max_stack_size".to_string(),
                DataComponent::MaxStackSize(max_stack_size),
            );
        }
        if let Some(md) = max_damage {
            components.insert(
                "minecraft:max_damage".to_string(),
                DataComponent::MaxDamage(md),
            );
        }
        if let Some(food) = food_properties {
            components.insert("minecraft:food".to_string(), DataComponent::Food(food));
            let (consume_seconds, animation, sound_override) = match loc.path.as_str() {
                "dried_kelp" => (0.8, "eat", None),
                "honey_bottle" => (2.0, "drink", Some("minecraft:item.honey_bottle.drink")),
                _ => (1.6, "eat", None),
            };
            components.insert(
                "minecraft:consumable".to_string(),
                vanilla_consumable_component(consume_seconds, animation, sound_override),
            );
        }
        if matches!(
            loc.path.as_str(),
            "milk_bucket" | "potion" | "ominous_bottle"
        ) {
            components.insert(
                "minecraft:consumable".to_string(),
                vanilla_consumable_component(1.6, "drink", None),
            );
        }
        if loc.namespace == "minecraft" && loc.path == "shield" {
            components.insert(
                "minecraft:blocks_attacks".to_string(),
                vanilla_shield_blocks_attacks_component(),
            );
        }
        if loc.namespace == "minecraft" && is_vanilla_bundle_item_path(&loc.path) {
            components.insert(
                "minecraft:bundle_contents".to_string(),
                DataComponent::Generic {
                    type_name: "minecraft:bundle_contents".to_string(),
                    value: Tag::List(Vec::new()),
                },
            );
        }
        if loc.namespace == "minecraft"
            && let Some(properties) = vanilla_spear_properties(&loc.path)
        {
            components.insert(
                "minecraft:swing_animation".to_string(),
                vanilla_spear_swing_animation_component(properties.swing_duration),
            );
            components.insert(
                "minecraft:kinetic_weapon".to_string(),
                vanilla_spear_kinetic_weapon_component(properties),
            );
            components.insert(
                "minecraft:piercing_weapon".to_string(),
                vanilla_spear_piercing_weapon_component(properties.uses_wood_sounds),
            );
            components.insert(
                "minecraft:attack_range".to_string(),
                vanilla_spear_attack_range_component(),
            );
            components.insert(
                "minecraft:minimum_attack_charge".to_string(),
                DataComponent::Generic {
                    type_name: "minecraft:minimum_attack_charge".to_string(),
                    value: Tag::Float(1.0),
                },
            );
        }
        let item = Item {
            location: loc.clone(),
            components: std::sync::Arc::new(components),
        };
        let registered_id = reg.register(loc, item);
        assert_eq!(
            registered_id, id,
            "Item ID mismatch during bootstrap for item {id}"
        );
    }
    reg
}
fn vanilla_consumable_component(
    consume_seconds: f32,
    animation: &str,
    sound_override: Option<&str>,
) -> DataComponent {
    let (sound, has_consume_particles) = if animation == "drink" {
        ("minecraft:entity.generic.drink", false)
    } else {
        ("minecraft:entity.generic.eat", true)
    };
    DataComponent::Generic {
        type_name: "minecraft:consumable".to_string(),
        value: Tag::Compound(vec![
            ("consume_seconds".to_string(), Tag::Float(consume_seconds)),
            ("animation".to_string(), Tag::String(animation.to_string())),
            (
                "sound".to_string(),
                Tag::String(sound_override.unwrap_or(sound).to_string()),
            ),
            (
                "has_consume_particles".to_string(),
                Tag::Byte(has_consume_particles as i8),
            ),
        ]),
    }
}
fn vanilla_shield_blocks_attacks_component() -> DataComponent {
    DataComponent::Generic {
        type_name: "minecraft:blocks_attacks".to_string(),
        value: Tag::Compound(vec![
            ("block_delay_seconds".to_string(), Tag::Float(0.25)),
            ("disable_cooldown_scale".to_string(), Tag::Float(1.0)),
            (
                "damage_reductions".to_string(),
                Tag::List(vec![Tag::Compound(vec![
                    ("horizontal_blocking_angle".to_string(), Tag::Float(90.0)),
                    ("base".to_string(), Tag::Float(0.0)),
                    ("factor".to_string(), Tag::Float(1.0)),
                ])]),
            ),
            (
                "item_damage".to_string(),
                Tag::Compound(vec![
                    ("threshold".to_string(), Tag::Float(3.0)),
                    ("base".to_string(), Tag::Float(1.0)),
                    ("factor".to_string(), Tag::Float(1.0)),
                ]),
            ),
        ]),
    }
}
#[derive(Clone, Copy)]
struct VanillaSpearProperties {
    swing_duration: i32,
    damage_multiplier: f32,
    delay_ticks: i32,
    dismount_duration_ticks: i32,
    dismount_min_speed: f32,
    knockback_duration_ticks: i32,
    knockback_min_speed: f32,
    damage_duration_ticks: i32,
    damage_min_relative_speed: f32,
    uses_wood_sounds: bool,
}
fn vanilla_spear_properties(path: &str) -> Option<VanillaSpearProperties> {
    match path {
        "wooden_spear" => Some(VanillaSpearProperties {
            swing_duration: 13,
            damage_multiplier: 0.7,
            delay_ticks: 15,
            dismount_duration_ticks: 100,
            dismount_min_speed: 14.0,
            knockback_duration_ticks: 200,
            knockback_min_speed: 5.1,
            damage_duration_ticks: 300,
            damage_min_relative_speed: 4.6,
            uses_wood_sounds: true,
        }),
        "stone_spear" => Some(VanillaSpearProperties {
            swing_duration: 15,
            damage_multiplier: 0.82,
            delay_ticks: 14,
            dismount_duration_ticks: 90,
            dismount_min_speed: 13.0,
            knockback_duration_ticks: 180,
            knockback_min_speed: 5.1,
            damage_duration_ticks: 275,
            damage_min_relative_speed: 4.6,
            uses_wood_sounds: false,
        }),
        "copper_spear" => Some(VanillaSpearProperties {
            swing_duration: 17,
            damage_multiplier: 0.82,
            delay_ticks: 13,
            dismount_duration_ticks: 80,
            dismount_min_speed: 12.0,
            knockback_duration_ticks: 165,
            knockback_min_speed: 5.1,
            damage_duration_ticks: 250,
            damage_min_relative_speed: 4.6,
            uses_wood_sounds: false,
        }),
        "iron_spear" => Some(VanillaSpearProperties {
            swing_duration: 19,
            damage_multiplier: 0.95,
            delay_ticks: 12,
            dismount_duration_ticks: 50,
            dismount_min_speed: 11.0,
            knockback_duration_ticks: 135,
            knockback_min_speed: 5.1,
            damage_duration_ticks: 225,
            damage_min_relative_speed: 4.6,
            uses_wood_sounds: false,
        }),
        "golden_spear" => Some(VanillaSpearProperties {
            swing_duration: 19,
            damage_multiplier: 0.7,
            delay_ticks: 14,
            dismount_duration_ticks: 70,
            dismount_min_speed: 13.0,
            knockback_duration_ticks: 170,
            knockback_min_speed: 5.1,
            damage_duration_ticks: 275,
            damage_min_relative_speed: 4.6,
            uses_wood_sounds: false,
        }),
        "diamond_spear" => Some(VanillaSpearProperties {
            swing_duration: 21,
            damage_multiplier: 1.075,
            delay_ticks: 10,
            dismount_duration_ticks: 60,
            dismount_min_speed: 10.0,
            knockback_duration_ticks: 130,
            knockback_min_speed: 5.1,
            damage_duration_ticks: 200,
            damage_min_relative_speed: 4.6,
            uses_wood_sounds: false,
        }),
        "netherite_spear" => Some(VanillaSpearProperties {
            swing_duration: 23,
            damage_multiplier: 1.2,
            delay_ticks: 8,
            dismount_duration_ticks: 50,
            dismount_min_speed: 9.0,
            knockback_duration_ticks: 110,
            knockback_min_speed: 5.1,
            damage_duration_ticks: 175,
            damage_min_relative_speed: 4.6,
            uses_wood_sounds: false,
        }),
        _ => None,
    }
}
fn vanilla_spear_swing_animation_component(duration: i32) -> DataComponent {
    DataComponent::Generic {
        type_name: "minecraft:swing_animation".to_string(),
        value: Tag::Compound(vec![
            ("type".to_string(), Tag::String("stab".to_string())),
            ("duration".to_string(), Tag::Int(duration)),
        ]),
    }
}
fn vanilla_spear_kinetic_weapon_component(properties: VanillaSpearProperties) -> DataComponent {
    let (sound, hit_sound) = if properties.uses_wood_sounds {
        ("minecraft:spear_wood_use", "minecraft:spear_wood_hit")
    } else {
        ("minecraft:spear_use", "minecraft:spear_hit")
    };
    DataComponent::Generic {
        type_name: "minecraft:kinetic_weapon".to_string(),
        value: Tag::Compound(vec![
            ("contact_cooldown_ticks".to_string(), Tag::Int(10)),
            ("delay_ticks".to_string(), Tag::Int(properties.delay_ticks)),
            (
                "dismount_conditions".to_string(),
                vanilla_spear_condition_component(
                    properties.dismount_duration_ticks,
                    properties.dismount_min_speed,
                    0.0,
                ),
            ),
            (
                "knockback_conditions".to_string(),
                vanilla_spear_condition_component(
                    properties.knockback_duration_ticks,
                    properties.knockback_min_speed,
                    0.0,
                ),
            ),
            (
                "damage_conditions".to_string(),
                vanilla_spear_condition_component(
                    properties.damage_duration_ticks,
                    0.0,
                    properties.damage_min_relative_speed,
                ),
            ),
            ("forward_movement".to_string(), Tag::Float(0.38)),
            (
                "damage_multiplier".to_string(),
                Tag::Float(properties.damage_multiplier),
            ),
            ("sound".to_string(), Tag::String(sound.to_string())),
            ("hit_sound".to_string(), Tag::String(hit_sound.to_string())),
        ]),
    }
}
fn vanilla_spear_condition_component(
    max_duration_ticks: i32,
    min_speed: f32,
    min_relative_speed: f32,
) -> Tag {
    Tag::Compound(vec![
        (
            "max_duration_ticks".to_string(),
            Tag::Int(max_duration_ticks),
        ),
        ("min_speed".to_string(), Tag::Float(min_speed)),
        (
            "min_relative_speed".to_string(),
            Tag::Float(min_relative_speed),
        ),
    ])
}
fn vanilla_spear_piercing_weapon_component(uses_wood_sounds: bool) -> DataComponent {
    let (sound, hit_sound) = if uses_wood_sounds {
        ("minecraft:spear_wood_attack", "minecraft:spear_wood_hit")
    } else {
        ("minecraft:spear_attack", "minecraft:spear_hit")
    };
    DataComponent::Generic {
        type_name: "minecraft:piercing_weapon".to_string(),
        value: Tag::Compound(vec![
            ("deals_knockback".to_string(), Tag::Byte(1)),
            ("dismounts".to_string(), Tag::Byte(0)),
            ("sound".to_string(), Tag::String(sound.to_string())),
            ("hit_sound".to_string(), Tag::String(hit_sound.to_string())),
        ]),
    }
}
fn vanilla_spear_attack_range_component() -> DataComponent {
    DataComponent::Generic {
        type_name: "minecraft:attack_range".to_string(),
        value: Tag::Compound(vec![
            ("min_reach".to_string(), Tag::Float(2.0)),
            ("max_reach".to_string(), Tag::Float(4.5)),
            ("min_creative_reach".to_string(), Tag::Float(2.0)),
            ("max_creative_reach".to_string(), Tag::Float(6.5)),
            ("hitbox_margin".to_string(), Tag::Float(0.125)),
            ("mob_factor".to_string(), Tag::Float(0.5)),
        ]),
    }
}
pub fn bootstrap_entity_types() -> DefaultedRegistry<EntityType> {
    let mut reg = DefaultedRegistry::new(ResourceLocation::from_string("minecraft:pig"));
    for &(id, name) in RAW_ENTITIES {
        let loc = ResourceLocation::from_string(name);
        let entity_type = EntityType {
            location: loc.clone(),
        };
        let registered_id = reg.register(loc, entity_type);
        assert_eq!(
            registered_id, id,
            "EntityType ID mismatch during bootstrap for {name}"
        );
    }
    reg
}
pub fn bootstrap_biomes() -> DefaultedRegistry<Biome> {
    let mut reg = DefaultedRegistry::new(ResourceLocation::from_string("minecraft:plains"));
    for &(id, name) in RAW_BIOMES {
        let loc = ResourceLocation::from_string(name);
        let mut spawners_map = HashMap::new();
        let creature_prob = 0.1;
        if spawners_map.is_empty() {
            spawners_map.insert(
                crate::biome::MobCategory::Monster,
                vec![
                    crate::biome::SpawnerData {
                        entity_type: ResourceLocation::from_string("minecraft:zombie"),
                        weight: 100,
                        min_count: 4,
                        max_count: 4,
                    },
                    crate::biome::SpawnerData {
                        entity_type: ResourceLocation::from_string("minecraft:skeleton"),
                        weight: 100,
                        min_count: 4,
                        max_count: 4,
                    },
                    crate::biome::SpawnerData {
                        entity_type: ResourceLocation::from_string("minecraft:creeper"),
                        weight: 100,
                        min_count: 4,
                        max_count: 4,
                    },
                    crate::biome::SpawnerData {
                        entity_type: ResourceLocation::from_string("minecraft:spider"),
                        weight: 100,
                        min_count: 4,
                        max_count: 4,
                    },
                ],
            );
            spawners_map.insert(
                crate::biome::MobCategory::Creature,
                vec![
                    crate::biome::SpawnerData {
                        entity_type: ResourceLocation::from_string("minecraft:sheep"),
                        weight: 12,
                        min_count: 4,
                        max_count: 4,
                    },
                    crate::biome::SpawnerData {
                        entity_type: ResourceLocation::from_string("minecraft:pig"),
                        weight: 10,
                        min_count: 4,
                        max_count: 4,
                    },
                    crate::biome::SpawnerData {
                        entity_type: ResourceLocation::from_string("minecraft:chicken"),
                        weight: 10,
                        min_count: 4,
                        max_count: 4,
                    },
                    crate::biome::SpawnerData {
                        entity_type: ResourceLocation::from_string("minecraft:cow"),
                        weight: 8,
                        min_count: 4,
                        max_count: 4,
                    },
                ],
            );
        }
        let spawn_settings = crate::biome::MobSpawnSettings {
            spawners: spawners_map,
            creature_spawn_probability: creature_prob,
        };
        let biome = Biome {
            location: loc.clone(),
            spawn_settings,
        };
        let registered_id = reg.register(loc, biome);
        assert_eq!(
            registered_id, id,
            "Biome ID mismatch during bootstrap for {name}"
        );
    }
    reg
}
pub struct BuiltInRegistries;
impl BuiltInRegistries {
    pub fn init() {
        BLOCK_STATE_REGISTRY.get_or_init(|| BlockStateRegistry::load());
        BLOCK_REGISTRY.get_or_init(|| bootstrap_blocks());
        ITEM_REGISTRY.get_or_init(|| bootstrap_items());
        ENTITY_TYPE_REGISTRY.get_or_init(|| bootstrap_entity_types());
        BIOME_REGISTRY.get_or_init(|| bootstrap_biomes());
    }
    pub fn block_states() -> &'static BlockStateRegistry {
        BLOCK_STATE_REGISTRY.get_or_init(|| BlockStateRegistry::load())
    }
    pub fn get_state_by_id(id: usize) -> Option<&'static BlockState> {
        Self::block_states().get_state_by_id(id)
    }
    pub fn all_block_states() -> &'static [BlockState] {
        Self::block_states().states()
    }
    pub fn get_id_by_state(
        block_name: &ResourceLocation,
        properties: &HashMap<String, String>,
    ) -> Option<usize> {
        Self::block_states().get_id_by_state(block_name, properties)
    }
    pub fn blocks() -> &'static DefaultedRegistry<Block> {
        let _ = Self::block_states();
        BLOCK_REGISTRY.get_or_init(|| bootstrap_blocks())
    }
    pub fn items() -> &'static DefaultedRegistry<Item> {
        ITEM_REGISTRY.get_or_init(|| bootstrap_items())
    }
    pub fn entity_types() -> &'static DefaultedRegistry<EntityType> {
        ENTITY_TYPE_REGISTRY.get_or_init(|| bootstrap_entity_types())
    }
    pub fn biomes() -> &'static DefaultedRegistry<Biome> {
        BIOME_REGISTRY.get_or_init(|| bootstrap_biomes())
    }
}
pub struct TagLoader {
    tags: HashMap<ResourceLocation, Vec<ResourceLocation>>,
}
impl TagLoader {
    pub fn new() -> Self {
        TagLoader {
            tags: HashMap::new(),
        }
    }
    pub fn add_tag(&mut self, location: ResourceLocation, entries: Vec<ResourceLocation>) {
        self.tags.insert(location, entries);
    }
    pub fn get_tag_entries(&self, location: &ResourceLocation) -> Option<&Vec<ResourceLocation>> {
        self.tags.get(location)
    }
    pub fn load_mock_tags_for_registry<T>(
        &self,
        tag_key: &TagKey<T>,
    ) -> Option<Vec<ResourceLocation>> {
        self.tags.get(&tag_key.location).cloned()
    }
}
