use std::collections::HashMap;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangedAttribute {
    pub id: &'static str,
    pub default_value: f64,
    pub min_value: f64,
    pub max_value: f64,
    pub syncable: bool,
}
impl RangedAttribute {
    pub const fn clamp(self, value: f64) -> f64 {
        if value.is_nan() || value < self.min_value {
            self.min_value
        } else if value > self.max_value {
            self.max_value
        } else {
            value
        }
    }
    pub fn clamp_f32(self, value: f32) -> f32 {
        self.clamp(value as f64) as f32
    }
}
pub const CAMERA_DISTANCE: RangedAttribute = RangedAttribute {
    id: "minecraft:camera_distance",
    default_value: 4.0,
    min_value: 0.0,
    max_value: 32.0,
    syncable: true,
};
pub const BELOW_NAME_DISTANCE: RangedAttribute = RangedAttribute {
    id: "minecraft:below_name_distance",
    default_value: 10.0,
    min_value: 0.0,
    max_value: 512.0,
    syncable: true,
};
pub const ARMOR: RangedAttribute = RangedAttribute {
    id: "minecraft:armor",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 30.0,
    syncable: true,
};
pub const ARMOR_TOUGHNESS: RangedAttribute = RangedAttribute {
    id: "minecraft:armor_toughness",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 20.0,
    syncable: true,
};
pub const LUCK: RangedAttribute = RangedAttribute {
    id: "minecraft:luck",
    default_value: 0.0,
    min_value: -1024.0,
    max_value: 1024.0,
    syncable: true,
};
pub const SWEEPING_DAMAGE_RATIO: RangedAttribute = RangedAttribute {
    id: "minecraft:sweeping_damage_ratio",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 1.0,
    syncable: true,
};
pub const WAYPOINT_TRANSMIT_RANGE: RangedAttribute = RangedAttribute {
    id: "minecraft:waypoint_transmit_range",
    default_value: 6.0e7,
    min_value: 0.0,
    max_value: 6.0e7,
    syncable: false,
};
pub const WAYPOINT_RECEIVE_RANGE: RangedAttribute = RangedAttribute {
    id: "minecraft:waypoint_receive_range",
    default_value: 6.0e7,
    min_value: 0.0,
    max_value: 6.0e7,
    syncable: false,
};
pub const MAX_ABSORPTION: RangedAttribute = RangedAttribute {
    id: "minecraft:max_absorption",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 2048.0,
    syncable: true,
};
pub const MAX_HEALTH: RangedAttribute = RangedAttribute {
    id: "minecraft:max_health",
    default_value: 20.0,
    min_value: 1.0,
    max_value: 1024.0,
    syncable: true,
};
pub const MOVEMENT_SPEED: RangedAttribute = RangedAttribute {
    id: "minecraft:movement_speed",
    default_value: 0.10000000149011612,
    min_value: 0.0,
    max_value: 1024.0,
    syncable: true,
};
pub const NAME_TAG_DISTANCE: RangedAttribute = RangedAttribute {
    id: "minecraft:name_tag_distance",
    default_value: 64.0,
    min_value: 0.0,
    max_value: 512.0,
    syncable: true,
};
pub const GRAVITY: RangedAttribute = RangedAttribute {
    id: "minecraft:gravity",
    default_value: 0.08,
    min_value: -1.0,
    max_value: 1.0,
    syncable: true,
};
pub const JUMP_STRENGTH: RangedAttribute = RangedAttribute {
    id: "minecraft:jump_strength",
    default_value: 0.41999998688697815,
    min_value: 0.0,
    max_value: 32.0,
    syncable: true,
};
pub const STEP_HEIGHT: RangedAttribute = RangedAttribute {
    id: "minecraft:step_height",
    default_value: 0.6,
    min_value: 0.0,
    max_value: 10.0,
    syncable: true,
};
pub const BLOCK_INTERACTION_RANGE: RangedAttribute = RangedAttribute {
    id: "minecraft:block_interaction_range",
    default_value: 4.5,
    min_value: 0.0,
    max_value: 64.0,
    syncable: true,
};
pub const ENTITY_INTERACTION_RANGE: RangedAttribute = RangedAttribute {
    id: "minecraft:entity_interaction_range",
    default_value: 3.0,
    min_value: 0.0,
    max_value: 64.0,
    syncable: true,
};
pub const SNEAKING_SPEED: RangedAttribute = RangedAttribute {
    id: "minecraft:sneaking_speed",
    default_value: 0.3,
    min_value: 0.0,
    max_value: 1.0,
    syncable: true,
};
pub const KNOCKBACK_RESISTANCE: RangedAttribute = RangedAttribute {
    id: "minecraft:knockback_resistance",
    default_value: 0.0,
    min_value: -2.0,
    max_value: 1.0,
    syncable: false,
};
pub const BOUNCINESS: RangedAttribute = RangedAttribute {
    id: "minecraft:bounciness",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 1.0,
    syncable: true,
};
pub const BURNING_TIME: RangedAttribute = RangedAttribute {
    id: "minecraft:burning_time",
    default_value: 1.0,
    min_value: 0.0,
    max_value: 1024.0,
    syncable: true,
};
pub const EXPLOSION_KNOCKBACK_RESISTANCE: RangedAttribute = RangedAttribute {
    id: "minecraft:explosion_knockback_resistance",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 1.0,
    syncable: true,
};
pub const FALL_DAMAGE_MULTIPLIER: RangedAttribute = RangedAttribute {
    id: "minecraft:fall_damage_multiplier",
    default_value: 1.0,
    min_value: 0.0,
    max_value: 100.0,
    syncable: true,
};
pub const OXYGEN_BONUS: RangedAttribute = RangedAttribute {
    id: "minecraft:oxygen_bonus",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 1024.0,
    syncable: true,
};
pub const SAFE_FALL_DISTANCE: RangedAttribute = RangedAttribute {
    id: "minecraft:safe_fall_distance",
    default_value: 3.0,
    min_value: -1024.0,
    max_value: 1024.0,
    syncable: true,
};
pub const SCALE: RangedAttribute = RangedAttribute {
    id: "minecraft:scale",
    default_value: 1.0,
    min_value: 0.0625,
    max_value: 16.0,
    syncable: true,
};
pub const FLYING_SPEED: RangedAttribute = RangedAttribute {
    id: "minecraft:flying_speed",
    default_value: 0.4,
    min_value: 0.0,
    max_value: 1024.0,
    syncable: true,
};
pub const WATER_MOVEMENT_EFFICIENCY: RangedAttribute = RangedAttribute {
    id: "minecraft:water_movement_efficiency",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 1.0,
    syncable: true,
};
pub const MOVEMENT_EFFICIENCY: RangedAttribute = RangedAttribute {
    id: "minecraft:movement_efficiency",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 1.0,
    syncable: true,
};
pub const AIR_DRAG_MODIFIER: RangedAttribute = RangedAttribute {
    id: "minecraft:air_drag_modifier",
    default_value: 1.0,
    min_value: 0.0,
    max_value: 2048.0,
    syncable: true,
};
pub const FRICTION_MODIFIER: RangedAttribute = RangedAttribute {
    id: "minecraft:friction_modifier",
    default_value: 1.0,
    min_value: 0.0,
    max_value: 2048.0,
    syncable: true,
};
pub const BLOCK_BREAK_SPEED: RangedAttribute = RangedAttribute {
    id: "minecraft:block_break_speed",
    default_value: 1.0,
    min_value: 0.0,
    max_value: 1024.0,
    syncable: true,
};
pub const MINING_EFFICIENCY: RangedAttribute = RangedAttribute {
    id: "minecraft:mining_efficiency",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 1024.0,
    syncable: true,
};
pub const SUBMERGED_MINING_SPEED: RangedAttribute = RangedAttribute {
    id: "minecraft:submerged_mining_speed",
    default_value: 0.2,
    min_value: 0.0,
    max_value: 20.0,
    syncable: true,
};
pub const ATTACK_DAMAGE: RangedAttribute = RangedAttribute {
    id: "minecraft:attack_damage",
    default_value: 2.0,
    min_value: 0.0,
    max_value: 2048.0,
    syncable: true,
};
pub const ATTACK_SPEED: RangedAttribute = RangedAttribute {
    id: "minecraft:attack_speed",
    default_value: 4.0,
    min_value: 0.0,
    max_value: 1024.0,
    syncable: true,
};
pub const ATTACK_KNOCKBACK: RangedAttribute = RangedAttribute {
    id: "minecraft:attack_knockback",
    default_value: 0.0,
    min_value: 0.0,
    max_value: 5.0,
    syncable: true,
};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeModifierOperation {
    AddValue,
    AddMultipliedBase,
    AddMultipliedTotal,
}
impl AttributeModifierOperation {
    pub fn from_wire(name: &str) -> Option<Self> {
        match name {
            "add_value" | "ADD_VALUE" | "0" => Some(Self::AddValue),
            "add_multiplied_base" | "ADD_MULTIPLIED_BASE" | "1" => Some(Self::AddMultipliedBase),
            "add_multiplied_total" | "ADD_MULTIPLIED_TOTAL" | "2" => Some(Self::AddMultipliedTotal),
            _ => None,
        }
    }
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            0 => Some(Self::AddValue),
            1 => Some(Self::AddMultipliedBase),
            2 => Some(Self::AddMultipliedTotal),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeModifier {
    pub id: String,
    pub amount: f64,
    pub operation: AttributeModifierOperation,
}
#[derive(Debug, Clone)]
pub struct AttributeInstance {
    pub definition: RangedAttribute,
    pub base: f64,
    pub modifiers: Vec<AttributeModifier>,
}
impl AttributeInstance {
    pub fn new(definition: RangedAttribute) -> Self {
        Self {
            definition,
            base: definition.default_value,
            modifiers: Vec::new(),
        }
    }
    pub fn set_base(&mut self, base: f64) {
        self.base = base;
    }
    pub fn remove_modifiers(&mut self) {
        self.modifiers.clear();
    }
    pub fn remove_modifier(&mut self, id: &str) {
        self.modifiers.retain(|m| m.id != id);
    }
    pub fn add_transient_modifier(&mut self, modifier: AttributeModifier) {
        self.modifiers.retain(|m| m.id != modifier.id);
        self.modifiers.push(modifier);
    }
    pub fn value(&self) -> f64 {
        let mut base = self.base;
        for m in &self.modifiers {
            if m.operation == AttributeModifierOperation::AddValue {
                base += m.amount;
            }
        }
        let mut result = base;
        for m in &self.modifiers {
            if m.operation == AttributeModifierOperation::AddMultipliedBase {
                result += base * m.amount;
            }
        }
        for m in &self.modifiers {
            if m.operation == AttributeModifierOperation::AddMultipliedTotal {
                result *= 1.0 + m.amount;
            }
        }
        self.definition.clamp(result)
    }
}
#[derive(Debug, Clone)]
pub struct AttributeMap {
    instances: HashMap<&'static str, AttributeInstance>,
}
impl AttributeMap {
    pub fn living_camera_defaults() -> Self {
        let mut map = Self {
            instances: HashMap::new(),
        };
        map.insert(SCALE);
        map.insert(CAMERA_DISTANCE);
        map
    }
    pub fn player_defaults() -> Self {
        let mut map = Self {
            instances: HashMap::new(),
        };
        map.insert(ARMOR);
        map.insert(ARMOR_TOUGHNESS);
        map.insert(LUCK);
        map.insert(SWEEPING_DAMAGE_RATIO);
        map.insert(WAYPOINT_TRANSMIT_RANGE);
        map.insert(WAYPOINT_RECEIVE_RANGE);
        map.insert(MAX_ABSORPTION);
        map.insert(MAX_HEALTH);
        map.insert(MOVEMENT_SPEED);
        map.insert(GRAVITY);
        map.insert(JUMP_STRENGTH);
        map.insert(STEP_HEIGHT);
        map.insert(BLOCK_INTERACTION_RANGE);
        map.insert(ENTITY_INTERACTION_RANGE);
        map.insert(SNEAKING_SPEED);
        map.insert(KNOCKBACK_RESISTANCE);
        map.insert(BURNING_TIME);
        map.insert(EXPLOSION_KNOCKBACK_RESISTANCE);
        map.insert(FALL_DAMAGE_MULTIPLIER);
        map.insert(OXYGEN_BONUS);
        map.insert(SAFE_FALL_DISTANCE);
        map.insert(SCALE);
        map.insert(FLYING_SPEED);
        map.insert(NAME_TAG_DISTANCE);
        map.insert(BELOW_NAME_DISTANCE);
        map.insert(WATER_MOVEMENT_EFFICIENCY);
        map.insert(MOVEMENT_EFFICIENCY);
        map.insert(AIR_DRAG_MODIFIER);
        map.insert(FRICTION_MODIFIER);
        map.insert(CAMERA_DISTANCE);
        map.insert(BLOCK_BREAK_SPEED);
        map.insert(MINING_EFFICIENCY);
        map.insert(SUBMERGED_MINING_SPEED);
        map.insert(ATTACK_DAMAGE);
        map.insert(ATTACK_SPEED);
        map.insert(ATTACK_KNOCKBACK);
        map.insert(BOUNCINESS);
        map
    }
    fn insert(&mut self, def: RangedAttribute) {
        self.instances.insert(def.id, AttributeInstance::new(def));
    }
    pub fn get(&self, id: &str) -> Option<&AttributeInstance> {
        let key = normalize_attribute_id(id);
        self.instances.get(key).or_else(|| self.instances.get(id))
    }
    pub fn get_mut(&mut self, id: &str) -> Option<&mut AttributeInstance> {
        let key = normalize_attribute_id(id);
        if self.instances.contains_key(key) {
            return self.instances.get_mut(key);
        }
        self.instances.get_mut(id)
    }
    pub fn value(&self, def: RangedAttribute) -> f64 {
        self.get(def.id)
            .map(|i| i.value())
            .unwrap_or(def.default_value)
    }
    pub fn value_f32(&self, def: RangedAttribute) -> f32 {
        self.value(def) as f32
    }
    pub fn apply_snapshot(
        &mut self,
        attribute_id: &str,
        base: f64,
        modifiers: Vec<AttributeModifier>,
    ) -> bool {
        if let Some(instance) = self.get_mut(attribute_id) {
            instance.set_base(base);
            instance.remove_modifiers();
            for m in modifiers {
                instance.add_transient_modifier(m);
            }
            true
        } else {
            false
        }
    }
    pub fn remove_modifier(&mut self, attribute_id: &str, modifier_id: &str) {
        if let Some(instance) = self.get_mut(attribute_id) {
            instance.remove_modifier(modifier_id);
        }
    }
    pub fn assign_all_values(&mut self, other: &AttributeMap) {
        for (id, other_instance) in &other.instances {
            if let Some(instance) = self.instances.get_mut(id) {
                instance.base = other_instance.base;
                instance.modifiers = other_instance.modifiers.clone();
            }
        }
    }
    pub fn assign_base_values(&mut self, other: &AttributeMap) {
        for (id, other_instance) in &other.instances {
            if let Some(instance) = self.instances.get_mut(id) {
                instance.base = other_instance.base;
                instance.modifiers.clear();
            }
        }
    }
    pub fn clear_all_modifiers(&mut self) {
        for instance in self.instances.values_mut() {
            instance.modifiers.clear();
        }
    }
}
fn normalize_attribute_id(id: &str) -> &str {
    let id = id.trim();
    if let Some(rest) = id.strip_prefix("minecraft:") {
        match rest {
            "movement_speed" => "minecraft:movement_speed",
            "armor" => "minecraft:armor",
            "armor_toughness" => "minecraft:armor_toughness",
            "luck" => "minecraft:luck",
            "sweeping_damage_ratio" => "minecraft:sweeping_damage_ratio",
            "waypoint_transmit_range" => "minecraft:waypoint_transmit_range",
            "waypoint_receive_range" => "minecraft:waypoint_receive_range",
            "max_absorption" => "minecraft:max_absorption",
            "max_health" => "minecraft:max_health",
            "gravity" => "minecraft:gravity",
            "jump_strength" => "minecraft:jump_strength",
            "step_height" => "minecraft:step_height",
            "block_interaction_range" => "minecraft:block_interaction_range",
            "entity_interaction_range" => "minecraft:entity_interaction_range",
            "sneaking_speed" => "minecraft:sneaking_speed",
            "knockback_resistance" => "minecraft:knockback_resistance",
            "scale" => "minecraft:scale",
            "flying_speed" => "minecraft:flying_speed",
            "water_movement_efficiency" => "minecraft:water_movement_efficiency",
            "movement_efficiency" => "minecraft:movement_efficiency",
            "air_drag_modifier" => "minecraft:air_drag_modifier",
            "friction_modifier" => "minecraft:friction_modifier",
            "camera_distance" => "minecraft:camera_distance",
            "below_name_distance" => "minecraft:below_name_distance",
            "name_tag_distance" => "minecraft:name_tag_distance",
            "block_break_speed" => "minecraft:block_break_speed",
            "mining_efficiency" => "minecraft:mining_efficiency",
            "submerged_mining_speed" => "minecraft:submerged_mining_speed",
            "attack_damage" => "minecraft:attack_damage",
            "attack_speed" => "minecraft:attack_speed",
            "attack_knockback" => "minecraft:attack_knockback",
            "bounciness" => "minecraft:bounciness",
            "burning_time" => "minecraft:burning_time",
            "explosion_knockback_resistance" => "minecraft:explosion_knockback_resistance",
            "fall_damage_multiplier" => "minecraft:fall_damage_multiplier",
            "oxygen_bonus" => "minecraft:oxygen_bonus",
            "safe_fall_distance" => "minecraft:safe_fall_distance",
            _ => id,
        }
    } else {
        match id {
            "movement_speed" => "minecraft:movement_speed",
            "armor" => "minecraft:armor",
            "armor_toughness" => "minecraft:armor_toughness",
            "luck" => "minecraft:luck",
            "sweeping_damage_ratio" => "minecraft:sweeping_damage_ratio",
            "waypoint_transmit_range" => "minecraft:waypoint_transmit_range",
            "waypoint_receive_range" => "minecraft:waypoint_receive_range",
            "max_absorption" => "minecraft:max_absorption",
            "max_health" => "minecraft:max_health",
            "gravity" => "minecraft:gravity",
            "jump_strength" => "minecraft:jump_strength",
            "step_height" => "minecraft:step_height",
            "block_interaction_range" => "minecraft:block_interaction_range",
            "entity_interaction_range" => "minecraft:entity_interaction_range",
            "sneaking_speed" => "minecraft:sneaking_speed",
            "knockback_resistance" => "minecraft:knockback_resistance",
            "scale" => "minecraft:scale",
            "flying_speed" => "minecraft:flying_speed",
            "water_movement_efficiency" => "minecraft:water_movement_efficiency",
            "movement_efficiency" => "minecraft:movement_efficiency",
            "air_drag_modifier" => "minecraft:air_drag_modifier",
            "friction_modifier" => "minecraft:friction_modifier",
            "camera_distance" => "minecraft:camera_distance",
            "below_name_distance" => "minecraft:below_name_distance",
            "name_tag_distance" => "minecraft:name_tag_distance",
            "block_break_speed" => "minecraft:block_break_speed",
            "mining_efficiency" => "minecraft:mining_efficiency",
            "submerged_mining_speed" => "minecraft:submerged_mining_speed",
            "attack_damage" => "minecraft:attack_damage",
            "attack_speed" => "minecraft:attack_speed",
            "attack_knockback" => "minecraft:attack_knockback",
            "bounciness" => "minecraft:bounciness",
            "burning_time" => "minecraft:burning_time",
            "explosion_knockback_resistance" => "minecraft:explosion_knockback_resistance",
            "fall_damage_multiplier" => "minecraft:fall_damage_multiplier",
            "oxygen_bonus" => "minecraft:oxygen_bonus",
            "safe_fall_distance" => "minecraft:safe_fall_distance",
            _ => id,
        }
    }
}
pub fn compute_modified_friction(friction: f32, modifier: f32) -> f32 {
    (1.0 - (1.0 - friction) * modifier).clamp(0.0, 1.0)
}
