use crate::chunk::{Chunk, ChunkSection};
use crate::paletted_container::PalettedContainer;
use el_core::block_state::BlockState;
use el_core::resource_location::ResourceLocation;
use el_nbt::{Tag, compound_get};
use std::sync::Arc;
pub const CHUNK_NBT_DATA_VERSION: i32 = 4903;
pub const CHUNK_NBT_FALLBACK_BIOME: &str = "minecraft:plains";
fn warn_invalid_resource_location(context: &str, value: &str) {
    eprintln!("el_world: {context}: invalid resource location {value:?}; entry skipped");
}
pub fn parse_chunk_from_nbt(tag: &Tag) -> Option<Chunk> {
    let root = match tag {
        Tag::Compound(fields) => fields,
        _ => return None,
    };
    let default_block = BlockState::new(Arc::new(ResourceLocation::from_string("minecraft:air")));
    let x_pos = match compound_get(root, "xPos") {
        Some(Tag::Int(x)) => *x,
        _ => 0,
    };
    let z_pos = match compound_get(root, "zPos") {
        Some(Tag::Int(z)) => *z,
        _ => 0,
    };
    let mut chunk = Chunk::new(x_pos, z_pos, -4, 19, default_block.clone());
    let sections_tag = compound_get(root, "sections")?;
    let sections_list = match sections_tag {
        Tag::List(list) => list,
        _ => return None,
    };
    for section_tag in sections_list {
        let section_fields = match section_tag {
            Tag::Compound(fields) => fields,
            _ => continue,
        };
        let y_index = match compound_get(section_fields, "Y") {
            Some(Tag::Byte(y)) => *y as i32,
            _ => continue,
        };
        let mut section = ChunkSection::new(y_index, default_block.clone());
        if let Some(Tag::Compound(block_fields)) = compound_get(section_fields, "block_states") {
            let palette = extract_palette(block_fields);
            let bits_per_entry = std::cmp::max(4, (palette.len() as f32).log2().ceil() as usize);
            let blocks = if let Some(Tag::LongArray(data)) = compound_get(block_fields, "data") {
                let u64_data: Vec<u64> = data.iter().map(|&v| v as u64).collect();
                PalettedContainer::from_raw(
                    palette,
                    u64_data,
                    bits_per_entry,
                    4096,
                    default_block.clone(),
                )
            } else {
                let values_per_long = 64 / bits_per_entry;
                let num_longs = (4096 + values_per_long - 1) / values_per_long;
                let storage = vec![0; num_longs];
                PalettedContainer::from_raw(
                    palette,
                    storage,
                    bits_per_entry,
                    4096,
                    default_block.clone(),
                )
            };
            section.blocks = blocks;
        }
        let chunk_y_offset = y_index - chunk.min_y_section;
        if chunk_y_offset >= 0 && chunk_y_offset < chunk.sections.len() as i32 {
            chunk.sections[chunk_y_offset as usize] = section;
        }
    }
    Some(chunk)
}
pub fn serialize_chunk_to_nbt(chunk: &Chunk) -> Tag {
    let mut root: Vec<(String, Tag)> = Vec::new();
    root.push(("xPos".to_string(), Tag::Int(chunk.x)));
    root.push(("zPos".to_string(), Tag::Int(chunk.z)));
    root.push(("yPos".to_string(), Tag::Int(chunk.min_y_section * 16)));
    let mut sections_list = Vec::new();
    for section in &chunk.sections {
        let mut section_fields: Vec<(String, Tag)> = Vec::new();
        section_fields.push(("Y".to_string(), Tag::Byte(section.y_index as i8)));
        let mut block_fields: Vec<(String, Tag)> = Vec::new();
        let palette_tag = serialize_palette(&section.blocks.palette);
        block_fields.push(("palette".to_string(), palette_tag));
        if section.blocks.palette.len() > 1 {
            let data_i64: Vec<i64> = section.blocks.storage.iter().map(|&v| v as i64).collect();
            block_fields.push(("data".to_string(), Tag::LongArray(data_i64)));
        }
        section_fields.push(("block_states".to_string(), Tag::Compound(block_fields)));
        let mut biomes_fields: Vec<(String, Tag)> = Vec::new();
        let mut biome_palette: Vec<Tag> = Vec::new();
        let mut bp_fields: Vec<(String, Tag)> = Vec::new();
        bp_fields.push((
            "Name".to_string(),
            Tag::String(CHUNK_NBT_FALLBACK_BIOME.to_string()),
        ));
        biome_palette.push(Tag::Compound(bp_fields));
        biomes_fields.push(("palette".to_string(), Tag::List(biome_palette)));
        section_fields.push(("biomes".to_string(), Tag::Compound(biomes_fields)));
        sections_list.push(Tag::Compound(section_fields));
    }
    root.push(("sections".to_string(), Tag::List(sections_list)));
    root.push(("DataVersion".to_string(), Tag::Int(CHUNK_NBT_DATA_VERSION)));
    Tag::Compound(root)
}
fn extract_palette(block_states: &Vec<(String, Tag)>) -> Vec<BlockState> {
    let mut palette = Vec::new();
    if let Some(Tag::List(palette_list)) = compound_get(block_states, "palette") {
        for item in palette_list {
            if let Tag::Compound(block_fields) = item {
                if let Some(Tag::String(name)) = compound_get(block_fields, "Name") {
                    let Some(loc) = ResourceLocation::try_from_string(name) else {
                        warn_invalid_resource_location("chunk block palette", name);
                        continue;
                    };
                    let mut state = BlockState::new(Arc::new(loc));
                    if let Some(Tag::Compound(props_fields)) =
                        compound_get(block_fields, "Properties")
                    {
                        for (k, v) in props_fields {
                            if let Tag::String(val) = v {
                                state = state.with_property(k, val);
                            }
                        }
                    }
                    palette.push(state);
                }
            }
        }
    }
    if palette.is_empty() {
        if let Some(air) = ResourceLocation::try_from_string("minecraft:air") {
            palette.push(BlockState::new(Arc::new(air)));
        }
    }
    palette
}
fn serialize_palette(palette: &[BlockState]) -> Tag {
    let mut list = Vec::new();
    for state in palette {
        let mut block_fields: Vec<(String, Tag)> = Vec::new();
        block_fields.push(("Name".to_string(), Tag::String(state.block.to_string())));
        if !state.properties.is_empty() {
            let mut props_fields: Vec<(String, Tag)> = Vec::new();
            for (k, v) in &state.properties {
                props_fields.push((k.clone(), Tag::String(v.clone())));
            }
            block_fields.push(("Properties".to_string(), Tag::Compound(props_fields)));
        }
        list.push(Tag::Compound(block_fields));
    }
    Tag::List(list)
}
fn uuid_to_ints(uuid: &uuid::Uuid) -> Vec<i32> {
    let bytes = uuid.as_bytes();
    vec![
        i32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        i32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        i32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]),
    ]
}
fn ints_to_uuid(ints: &[i32]) -> Option<uuid::Uuid> {
    if ints.len() != 4 {
        return None;
    }
    let mut bytes = [0u8; 16];
    bytes[0..4].copy_from_slice(&ints[0].to_be_bytes());
    bytes[4..8].copy_from_slice(&ints[1].to_be_bytes());
    bytes[8..12].copy_from_slice(&ints[2].to_be_bytes());
    bytes[12..16].copy_from_slice(&ints[3].to_be_bytes());
    Some(uuid::Uuid::from_bytes(bytes))
}
pub fn serialize_entity_to_nbt(entity: &el_core::entity::Entity) -> Tag {
    let mut map: Vec<(String, Tag)> = Vec::new();
    map.push((
        "id".to_string(),
        Tag::String(entity.entity_type.to_string()),
    ));
    map.push((
        "Pos".to_string(),
        Tag::List(vec![
            Tag::Double(entity.x),
            Tag::Double(entity.y),
            Tag::Double(entity.z),
        ]),
    ));
    map.push((
        "Motion".to_string(),
        Tag::List(vec![
            Tag::Double(entity.motion_x),
            Tag::Double(entity.motion_y),
            Tag::Double(entity.motion_z),
        ]),
    ));
    map.push((
        "Rotation".to_string(),
        Tag::List(vec![Tag::Float(entity.y_rot), Tag::Float(entity.x_rot)]),
    ));
    map.push((
        "UUID".to_string(),
        Tag::IntArray(uuid_to_ints(&entity.uuid)),
    ));
    map.push((
        "OnGround".to_string(),
        Tag::Byte(if entity.on_ground { 1 } else { 0 }),
    ));
    map.push((
        "NoAI".to_string(),
        Tag::Byte(if entity.no_ai { 1 } else { 0 }),
    ));
    map.push(("Air".to_string(), Tag::Short(300)));
    map.push(("Fire".to_string(), Tag::Short(0)));
    map.push(("FallFlying".to_string(), Tag::Byte(0)));
    map.push(("Invulnerable".to_string(), Tag::Byte(0)));
    map.push(("PortalCooldown".to_string(), Tag::Int(0)));
    map.push(("fall_distance".to_string(), Tag::Double(0.0)));
    let mut has_health = false;
    match &entity.kind {
        el_core::entity::EntityKind::Zombie { health } => {
            map.push(("Health".to_string(), Tag::Float(*health)));
            map.push(("AbsorptionAmount".to_string(), Tag::Float(0.0)));
            map.push(("DeathTime".to_string(), Tag::Short(0)));
            map.push(("HurtTime".to_string(), Tag::Short(0)));
            map.push(("IsBaby".to_string(), Tag::Byte(0)));
            map.push(("CanBreakDoors".to_string(), Tag::Byte(0)));
            map.push(("DrownedConversionTime".to_string(), Tag::Int(-1)));
            map.push(("InWaterTime".to_string(), Tag::Int(-1)));
            has_health = true;
        }
        el_core::entity::EntityKind::Skeleton { health } => {
            map.push(("Health".to_string(), Tag::Float(*health)));
            map.push(("AbsorptionAmount".to_string(), Tag::Float(0.0)));
            map.push(("DeathTime".to_string(), Tag::Short(0)));
            map.push(("HurtTime".to_string(), Tag::Short(0)));
            map.push(("StrayConversionTime".to_string(), Tag::Int(-1)));
            has_health = true;
        }
        el_core::entity::EntityKind::Creeper {
            health,
            fuse,
            is_ignited,
        } => {
            map.push(("Health".to_string(), Tag::Float(*health)));
            map.push(("AbsorptionAmount".to_string(), Tag::Float(0.0)));
            map.push(("DeathTime".to_string(), Tag::Short(0)));
            map.push(("HurtTime".to_string(), Tag::Short(0)));
            map.push(("Fuse".to_string(), Tag::Short(*fuse as i16)));
            map.push(("ExplosionRadius".to_string(), Tag::Byte(3)));
            map.push((
                "ignited".to_string(),
                Tag::Byte(if *is_ignited { 1 } else { 0 }),
            ));
            map.push(("powered".to_string(), Tag::Byte(0)));
            has_health = true;
        }
        el_core::entity::EntityKind::Item {
            item_stack,
            pickup_delay,
        } => {
            map.push(("Item".to_string(), item_stack.to_nbt()));
            map.push(("Age".to_string(), Tag::Short(0)));
            map.push(("PickupDelay".to_string(), Tag::Short(*pickup_delay)));
            map.push(("Health".to_string(), Tag::Short(5)));
        }
        el_core::entity::EntityKind::Player => {
            let mut inv_list = Vec::new();
            for (slot_idx, stack) in entity.inventory.iter().enumerate() {
                if !stack.is_empty() {
                    let mut item_tag = match stack.to_nbt() {
                        Tag::Compound(m) => m,
                        _ => continue,
                    };
                    item_tag.push(("Slot".to_string(), Tag::Byte(slot_idx as i8)));
                    inv_list.push(Tag::Compound(item_tag));
                }
            }
            map.push(("Inventory".to_string(), Tag::List(inv_list)));
            map.push(("Health".to_string(), Tag::Float(entity.health)));
            map.push((
                "playerGameType".to_string(),
                Tag::Int(entity.game_mode as i32),
            ));
            map.push(("foodLevel".to_string(), Tag::Int(entity.food_level)));
            map.push((
                "foodSaturationLevel".to_string(),
                Tag::Float(entity.food_saturation),
            ));
            map.push((
                "foodExhaustionLevel".to_string(),
                Tag::Float(entity.food_exhaustion),
            ));
            map.push(("XpLevel".to_string(), Tag::Int(entity.xp_level)));
            map.push(("XpP".to_string(), Tag::Float(entity.xp_progress)));
            map.push(("XpTotal".to_string(), Tag::Int(entity.xp_total)));
        }
        _ => {}
    };
    if has_health {
        map.push(("CanPickUpLoot".to_string(), Tag::Byte(0)));
        map.push(("LeftHanded".to_string(), Tag::Byte(0)));
        map.push(("PersistenceRequired".to_string(), Tag::Byte(0)));
        map.push(("Brain".to_string(), Tag::Compound(Vec::new())));
        map.push(("attributes".to_string(), Tag::List(Vec::new())));
    }
    Tag::Compound(map)
}
pub fn deserialize_entity_from_nbt(tag: &Tag) -> Option<el_core::entity::Entity> {
    let map = match tag {
        Tag::Compound(m) => m,
        _ => return None,
    };
    let id_str = match compound_get(map, "id")? {
        Tag::String(s) => s,
        _ => return None,
    };
    let Some(entity_type) = ResourceLocation::try_from_string(id_str) else {
        warn_invalid_resource_location("chunk entity id", id_str);
        return None;
    };
    let pos_list = match compound_get(map, "Pos")? {
        Tag::List(l) => l,
        _ => return None,
    };
    let x = match pos_list.get(0)? {
        Tag::Double(v) => *v,
        _ => 0.0,
    };
    let y = match pos_list.get(1)? {
        Tag::Double(v) => *v,
        _ => 0.0,
    };
    let z = match pos_list.get(2)? {
        Tag::Double(v) => *v,
        _ => 0.0,
    };
    let (motion_x, motion_y, motion_z) =
        if let Some(Tag::List(mot_list)) = compound_get(map, "Motion") {
            let mx = match mot_list.get(0) {
                Some(Tag::Double(v)) => *v,
                _ => 0.0,
            };
            let my = match mot_list.get(1) {
                Some(Tag::Double(v)) => *v,
                _ => 0.0,
            };
            let mz = match mot_list.get(2) {
                Some(Tag::Double(v)) => *v,
                _ => 0.0,
            };
            (mx, my, mz)
        } else {
            (0.0, 0.0, 0.0)
        };
    let (y_rot, x_rot) = if let Some(Tag::List(rot_list)) = compound_get(map, "Rotation") {
        let yr = match rot_list.get(0) {
            Some(Tag::Float(v)) => *v,
            _ => 0.0,
        };
        let xr = match rot_list.get(1) {
            Some(Tag::Float(v)) => *v,
            _ => 0.0,
        };
        (yr, xr)
    } else {
        (0.0, 0.0)
    };
    let uuid = match compound_get(map, "UUID") {
        Some(Tag::IntArray(arr)) => ints_to_uuid(arr).unwrap_or_else(uuid::Uuid::new_v4),
        _ => uuid::Uuid::new_v4(),
    };
    let on_ground = match compound_get(map, "OnGround") {
        Some(Tag::Byte(b)) => *b != 0,
        _ => false,
    };
    let kind = match id_str.as_str() {
        "minecraft:zombie" => {
            let health = match compound_get(map, "Health") {
                Some(Tag::Float(h)) => *h,
                _ => 20.0,
            };
            el_core::entity::EntityKind::Zombie { health }
        }
        "minecraft:skeleton" => {
            let health = match compound_get(map, "Health") {
                Some(Tag::Float(h)) => *h,
                _ => 20.0,
            };
            el_core::entity::EntityKind::Skeleton { health }
        }
        "minecraft:creeper" => {
            let health = match compound_get(map, "Health") {
                Some(Tag::Float(h)) => *h,
                _ => 20.0,
            };
            let fuse = match compound_get(map, "Fuse") {
                Some(Tag::Short(s)) => *s as i32,
                Some(Tag::Int(f)) => *f,
                _ => 30,
            };
            let ignited = match compound_get(map, "ignited") {
                Some(Tag::Byte(b)) => *b != 0,
                _ => false,
            };
            el_core::entity::EntityKind::Creeper {
                health,
                fuse,
                is_ignited: ignited,
            }
        }
        "minecraft:item" => {
            let item_tag = compound_get(map, "Item")?;
            let item_stack = el_core::ItemStack::from_nbt(item_tag)?;
            let pickup_delay = match compound_get(map, "PickupDelay") {
                Some(Tag::Short(s)) => *s,
                _ => 0,
            };
            el_core::entity::EntityKind::Item {
                item_stack,
                pickup_delay,
            }
        }
        _ => {
            if id_str == "minecraft:player" {
                el_core::entity::EntityKind::Player
            } else {
                eprintln!(
                    "el_world: chunk entity id {id_str:?} has no deserializer; entity skipped"
                );
                return None;
            }
        }
    };
    let mut entity = el_core::entity::Entity::new(0, uuid, entity_type, kind, x, y, z);
    entity.motion_x = motion_x;
    entity.motion_y = motion_y;
    entity.motion_z = motion_z;
    entity.y_rot = y_rot;
    entity.x_rot = x_rot;
    entity.on_ground = on_ground;
    let no_ai = match compound_get(map, "NoAI") {
        Some(Tag::Byte(b)) => *b != 0,
        _ => false,
    };
    entity.no_ai = no_ai;
    if id_str == "minecraft:player" {
        if let Some(Tag::Float(h)) = compound_get(map, "Health") {
            entity.health = *h;
        }
        if let Some(Tag::Int(g)) = compound_get(map, "playerGameType") {
            entity.game_mode = *g as u8;
        }
        if let Some(Tag::Int(f)) = compound_get(map, "foodLevel") {
            entity.food_level = *f;
        }
        if let Some(Tag::Float(s)) = compound_get(map, "foodSaturationLevel") {
            entity.food_saturation = *s;
        }
        if let Some(Tag::Float(e)) = compound_get(map, "foodExhaustionLevel") {
            entity.food_exhaustion = *e;
        }
        if let Some(Tag::Int(x)) = compound_get(map, "XpLevel") {
            entity.xp_level = *x;
        }
        if let Some(Tag::Float(p)) = compound_get(map, "XpP") {
            entity.xp_progress = *p;
        }
        if let Some(Tag::Int(t)) = compound_get(map, "XpTotal") {
            entity.xp_total = *t;
        }
        let mut inventory = vec![el_core::ItemStack::empty(); 46];
        if let Some(Tag::List(inv_list)) = compound_get(map, "Inventory") {
            for item_tag in inv_list {
                if let Tag::Compound(item_map) = item_tag {
                    if let Some(Tag::Byte(slot_idx)) = compound_get(item_map, "Slot") {
                        let idx = *slot_idx as usize;
                        if idx < inventory.len() {
                            if let Some(stack) = el_core::ItemStack::from_nbt(item_tag) {
                                inventory[idx] = stack;
                            }
                        }
                    }
                }
            }
        }
        entity.inventory = inventory;
    }
    Some(entity)
}
pub fn parse_entities_from_chunk_nbt(tag: &Tag) -> Vec<el_core::entity::Entity> {
    let mut entities = Vec::new();
    let root = match tag {
        Tag::Compound(fields) => fields,
        _ => return entities,
    };
    if let Some(Tag::List(list)) = compound_get(root, "Entities") {
        for ent_tag in list {
            if let Some(entity) = deserialize_entity_from_nbt(ent_tag) {
                entities.push(entity);
            }
        }
    }
    entities
}
pub fn serialize_chunk_to_nbt_with_entities(
    chunk: &Chunk,
    entities: &[el_core::entity::Entity],
) -> Tag {
    let tag = serialize_chunk_to_nbt(chunk);
    let mut root = match tag {
        Tag::Compound(fields) => fields,
        _ => return tag,
    };
    let mut entities_list = Vec::new();
    for entity in entities {
        entities_list.push(serialize_entity_to_nbt(entity));
    }
    root.push(("Entities".to_string(), Tag::List(entities_list)));
    Tag::Compound(root)
}
