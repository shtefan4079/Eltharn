use crate::item::ItemStack;
use crate::resource_location::ResourceLocation;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub enum EntityKind {
    Player,
    Item {
        item_stack: ItemStack,
        pickup_delay: i16,
    },
    Zombie {
        health: f32,
    },
    Skeleton {
        health: f32,
    },
    Creeper {
        health: f32,
        fuse: i32,
        is_ignited: bool,
    },
    PassiveAnimal {
        location: ResourceLocation,
        health: f32,
        age: i32,
    },
    Projectile {
        owner_uuid: Option<uuid::Uuid>,
    },
    ExperienceOrb {
        value: i32,
    },
}

impl EntityKind {
    pub fn category(&self) -> crate::biome::MobCategory {
        match self {
            EntityKind::Player => crate::biome::MobCategory::Misc,
            EntityKind::Item { .. } => crate::biome::MobCategory::Misc,
            EntityKind::Projectile { .. } => crate::biome::MobCategory::Misc,
            EntityKind::ExperienceOrb { .. } => crate::biome::MobCategory::Misc,
            EntityKind::Zombie { .. }
            | EntityKind::Skeleton { .. }
            | EntityKind::Creeper { .. } => crate::biome::MobCategory::Monster,
            EntityKind::PassiveAnimal { location, .. } => {
                let path = location.path.as_str();
                if path == "bat" {
                    crate::biome::MobCategory::Ambient
                } else if path == "glow_squid" {
                    crate::biome::MobCategory::UndergroundWaterCreature
                } else if path == "squid"
                    || path == "cod"
                    || path == "salmon"
                    || path == "pufferfish"
                    || path == "tropical_fish"
                    || path == "dolphin"
                {
                    crate::biome::MobCategory::WaterCreature
                } else if path == "axolotl" {
                    crate::biome::MobCategory::Axolotls
                } else {
                    crate::biome::MobCategory::Creature
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LookControl {
    pub wanted_x: f64,
    pub wanted_y: f64,
    pub wanted_z: f64,
    pub y_max_rot_speed: f32,
    pub x_max_rot_angle: f32,
    pub look_at_cooldown: i32,
}

impl Default for LookControl {
    fn default() -> Self {
        Self {
            wanted_x: 0.0,
            wanted_y: 0.0,
            wanted_z: 0.0,
            y_max_rot_speed: 10.0,
            x_max_rot_angle: 40.0,
            look_at_cooldown: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveControlOperation {
    Wait,
    MoveTo,
    Strafe,
    Jumping,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MoveControl {
    pub wanted_x: f64,
    pub wanted_y: f64,
    pub wanted_z: f64,
    pub speed_modifier: f64,
    pub operation: MoveControlOperation,
}

impl Default for MoveControl {
    fn default() -> Self {
        Self {
            wanted_x: 0.0,
            wanted_y: 0.0,
            wanted_z: 0.0,
            speed_modifier: 1.0,
            operation: MoveControlOperation::Wait,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PathNavigation {
    pub path: Vec<el_math::block_pos::BlockPos>,
    pub current_node_index: usize,
    pub speed_modifier: f64,
}

impl Default for PathNavigation {
    fn default() -> Self {
        Self {
            path: Vec::new(),
            current_node_index: 0,
            speed_modifier: 1.0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub id: i32,
    pub uuid: uuid::Uuid,
    pub entity_type: ResourceLocation,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub x_rot: f32,
    pub y_rot: f32,
    pub motion_x: f64,
    pub motion_y: f64,
    pub motion_z: f64,
    pub on_ground: bool,
    pub removed: bool,
    pub kind: EntityKind,
    pub metadata: SynchedEntityData,
    pub y_head_rot: f32,
    pub y_body_rot: f32,
    pub no_ai: bool,
    pub look_control: LookControl,
    pub move_control: MoveControl,
    pub path_navigation: PathNavigation,
    pub wander_target: Option<el_math::block_pos::BlockPos>,
    pub wander_cooldown: i32,
    pub name: String,
    pub game_mode: u8,
    pub inventory: Vec<ItemStack>,
    pub health: f32,
    pub max_health: f32,
    pub food_level: i32,
    pub food_saturation: f32,
    pub food_exhaustion: f32,
    pub air_supply: i32,
    pub fire_ticks: i32,
    pub xp_level: i32,
    pub xp_progress: f32,
    pub xp_total: i32,
    pub active_effects: HashMap<i32, i32>,
    pub fall_distance: f32,
    pub sleep_timer: i32,
    pub spawn_point: Option<el_math::block_pos::BlockPos>,
    pub stats: HashMap<String, i32>,
    pub completed_criteria: HashMap<ResourceLocation, HashSet<String>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EntityDataValue {
    Byte(i8),
    VarInt(i32),
    VarLong(i64),
    Float(f32),
    String(String),
    Component(String),
    OptionalComponent(Option<String>),
    ItemStack(ItemStack),
    Boolean(bool),
    Rotations(f32, f32, f32),
    BlockPos(el_math::block_pos::BlockPos),
    OptionalBlockPos(Option<el_math::block_pos::BlockPos>),
    Direction(i32),
    OptionalUUID(Option<uuid::Uuid>),
    BlockState(i32),
    OptionalBlockState(Option<i32>),
    NbtCompound(el_nbt::Tag),
    OptionalVarInt(Option<i32>),
    Pose(i32),
}

#[derive(Clone, Debug, PartialEq)]
pub struct SynchedEntityData {
    pub values: HashMap<u8, (i32, EntityDataValue)>,
}

impl SynchedEntityData {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
        }
    }

    pub fn set(&mut self, index: u8, serializer_id: i32, value: EntityDataValue) {
        self.values.insert(index, (serializer_id, value));
    }

    pub fn get(&self, index: u8) -> Option<&(i32, EntityDataValue)> {
        self.values.get(&index)
    }
}

impl Entity {
    pub fn new(
        id: i32,
        uuid: uuid::Uuid,
        entity_type: ResourceLocation,
        kind: EntityKind,
        x: f64,
        y: f64,
        z: f64,
    ) -> Self {
        let mut metadata = SynchedEntityData::new();
        match &kind {
            EntityKind::Zombie { health }
            | EntityKind::Skeleton { health }
            | EntityKind::Creeper { health, .. } => {
                metadata.set(9, 3, EntityDataValue::Float(*health));
            }
            EntityKind::Item { item_stack, .. } => {
                metadata.set(8, 7, EntityDataValue::ItemStack(item_stack.clone()));
            }
            _ => {}
        }

        let name = match &kind {
            EntityKind::Player => "Steve".to_string(),
            _ => entity_type.path.clone(),
        };

        let game_mode = match &kind {
            EntityKind::Player => 1,
            _ => 0,
        };

        let (health, max_health) = match &kind {
            EntityKind::Zombie { health }
            | EntityKind::Skeleton { health }
            | EntityKind::Creeper { health, .. }
            | EntityKind::PassiveAnimal { health, .. } => (*health, *health),
            EntityKind::Player => (20.0, 20.0),
            _ => (20.0, 20.0),
        };

        Self {
            id,
            uuid,
            entity_type,
            x,
            y,
            z,
            x_rot: 0.0,
            y_rot: 0.0,
            motion_x: 0.0,
            motion_y: 0.0,
            motion_z: 0.0,
            on_ground: false,
            removed: false,
            kind,
            metadata,
            y_head_rot: 0.0,
            y_body_rot: 0.0,
            no_ai: false,
            look_control: LookControl::default(),
            move_control: MoveControl::default(),
            path_navigation: PathNavigation::default(),
            wander_target: None,
            wander_cooldown: 0,
            name,
            game_mode,
            inventory: Vec::new(),
            health,
            max_health,
            food_level: 20,
            food_saturation: 5.0,
            food_exhaustion: 0.0,
            air_supply: 300,
            fire_ticks: 0,
            xp_level: 0,
            xp_progress: 0.0,
            xp_total: 0,
            active_effects: HashMap::new(),
            fall_distance: 0.0,
            sleep_timer: 0,
            spawn_point: None,
            stats: HashMap::new(),
            completed_criteria: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EntityType {
    pub location: ResourceLocation,
}
