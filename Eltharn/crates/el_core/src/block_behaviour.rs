use crate::block::Block;
use crate::block_state::BlockState;
use crate::collision_context::CollisionContext;
use el_math::block_pos::BlockPos;
use el_math::{AABB, VoxelShape};
#[allow(dead_code)]
mod generated_collision_shapes {
    include!("generated/generated_collision_shapes.rs");
}
#[allow(dead_code)]
mod generated_java_26_2_block_tags {
    include!("generated/generated_java_26_2_block_tags.rs");
}
#[allow(dead_code)]
mod generated_java_26_2_mining {
    include!("generated/generated_java_26_2_mining.rs");
}
#[allow(dead_code)]
mod generated_java_26_2_block_lighting {
    include!("generated/generated_java_26_2_block_lighting.rs");
}
#[allow(dead_code)]
mod generated_java_26_2_block_sound_type {
    include!("generated/generated_java_26_2_block_sound_type.rs");
}
#[allow(dead_code)]
mod generated_java_26_2_flammability {
    include!("generated/generated_java_26_2_flammability.rs");
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Java26_2BlockSoundType {
    pub volume: f32,
    pub pitch: f32,
    pub break_sound: &'static str,
    pub step_sound: &'static str,
    pub place_sound: &'static str,
    pub hit_sound: &'static str,
    pub fall_sound: &'static str,
}
impl Java26_2BlockSoundType {
    pub fn step_volume_and_pitch(self) -> (f32, f32) {
        (self.volume * 0.15, self.pitch)
    }
    pub fn hit_volume_and_pitch(self) -> (f32, f32) {
        ((self.volume + 1.0) / 8.0, self.pitch * 0.5)
    }
    pub fn break_volume_and_pitch(self) -> (f32, f32) {
        ((self.volume + 1.0) / 2.0, self.pitch * 0.8)
    }
}
#[allow(dead_code)]
mod generated_java_26_2_block_fluid_support {
    include!("generated/generated_java_26_2_block_fluid_support.rs");
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Java26_2BlockFluidSupportProperties {
    pub solid: bool,
    pub blocks_motion: bool,
    pub face_sturdy: [bool; 6],
    pub fluid_side_overlay: bool,
    pub ice_block: bool,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Java26_2BlockLightingProperties {
    pub light_emission: u8,
    pub emissive_rendering: bool,
    pub solid_render: bool,
    pub light_dampening: u8,
    pub view_blocking: bool,
    pub shade_brightness: f32,
}
pub fn is_java_26_2_vanilla_block_in_tag(block_id: &str, tag_id: &str) -> bool {
    let block_id = normalize_java_26_2_vanilla_resource_location(block_id);
    let tag_id = normalize_java_26_2_vanilla_resource_location(tag_id);
    generated_java_26_2_block_tags::contains_java_26_2_block_tag(&tag_id, &block_id)
}
pub fn java_26_2_vanilla_block_mining_properties(block_id: &str) -> Option<(f32, bool)> {
    let block_id = normalize_java_26_2_vanilla_resource_location(block_id);
    generated_java_26_2_mining::java_26_2_block_mining(&block_id)
}
pub fn java_26_2_vanilla_block_ignited_by_lava(block_id: &str) -> Option<bool> {
    let block_id = normalize_java_26_2_vanilla_resource_location(block_id);
    generated_java_26_2_flammability::java_26_2_block_ignited_by_lava(&block_id)
}
pub fn java_26_2_vanilla_block_fire_odds(block_id: &str) -> Option<(u16, u16)> {
    let block_id = normalize_java_26_2_vanilla_resource_location(block_id);
    generated_java_26_2_flammability::java_26_2_block_fire_odds(&block_id)
}
pub fn java_26_2_vanilla_block_lighting_properties(
    state: &BlockState,
) -> Option<Java26_2BlockLightingProperties> {
    if state.block.namespace != "minecraft" {
        return None;
    }
    let block_id = state.block.to_string();
    let entry = generated_java_26_2_block_lighting::java_26_2_block_lighting(state.id, &block_id)?;
    Some(Java26_2BlockLightingProperties {
        light_emission: entry.light_emission,
        emissive_rendering: entry.emissive_rendering,
        solid_render: entry.solid_render,
        light_dampening: entry.light_dampening,
        view_blocking: entry.view_blocking,
        shade_brightness: entry.shade_brightness,
    })
}
pub fn java_26_2_vanilla_block_sound_type(state: &BlockState) -> Option<Java26_2BlockSoundType> {
    if state.block.namespace != "minecraft" {
        return None;
    }
    let block_id = state.block.to_string();
    let entry =
        generated_java_26_2_block_sound_type::java_26_2_block_sound_type(state.id, &block_id)?;
    Some(Java26_2BlockSoundType {
        volume: entry.volume,
        pitch: entry.pitch,
        break_sound: entry.sounds[0],
        step_sound: entry.sounds[1],
        place_sound: entry.sounds[2],
        hit_sound: entry.sounds[3],
        fall_sound: entry.sounds[4],
    })
}
pub fn java_26_2_vanilla_block_fluid_support_properties(
    state: &BlockState,
) -> Option<Java26_2BlockFluidSupportProperties> {
    if state.block.namespace != "minecraft" {
        return None;
    }
    let block_id = state.block.to_string();
    let entry = generated_java_26_2_block_fluid_support::java_26_2_block_fluid_support(
        state.id, &block_id,
    )?;
    Some(Java26_2BlockFluidSupportProperties {
        solid: entry.solid,
        blocks_motion: entry.blocks_motion,
        face_sturdy: entry.face_sturdy,
        fluid_side_overlay: entry.fluid_side_overlay,
        ice_block: entry.ice_block,
    })
}
pub fn java_26_2_vanilla_block_render_shape_is_model(block_id: &str) -> Option<bool> {
    let block_id = normalize_java_26_2_vanilla_resource_location(block_id);
    generated_java_26_2_mining::java_26_2_block_render_shape_is_model(&block_id)
}
pub fn java_26_2_vanilla_block_has_menu_provider(state: &BlockState) -> Option<bool> {
    if state.block.namespace != "minecraft" {
        return None;
    }
    let name = state.block.path.as_str();
    let has_provider = match name {
        "anvil"
        | "chipped_anvil"
        | "damaged_anvil"
        | "cartography_table"
        | "crafting_table"
        | "enchanting_table"
        | "grindstone"
        | "loom"
        | "smithing_table"
        | "stonecutter"
        | "furnace"
        | "blast_furnace"
        | "smoker"
        | "barrel"
        | "brewing_stand"
        | "dispenser"
        | "dropper"
        | "hopper"
        | "beacon"
        | "chest"
        | "trapped_chest"
        | "ender_chest"
        | "copper_chest"
        | "exposed_copper_chest"
        | "weathered_copper_chest"
        | "oxidized_copper_chest"
        | "waxed_copper_chest"
        | "waxed_exposed_copper_chest"
        | "waxed_weathered_copper_chest"
        | "waxed_oxidized_copper_chest"
        | "shulker_box"
        | "white_shulker_box"
        | "orange_shulker_box"
        | "magenta_shulker_box"
        | "light_blue_shulker_box"
        | "yellow_shulker_box"
        | "lime_shulker_box"
        | "pink_shulker_box"
        | "gray_shulker_box"
        | "light_gray_shulker_box"
        | "cyan_shulker_box"
        | "purple_shulker_box"
        | "blue_shulker_box"
        | "brown_shulker_box"
        | "green_shulker_box"
        | "red_shulker_box"
        | "black_shulker_box"
        | "crafter"
        | "command_block"
        | "chain_command_block"
        | "repeating_command_block" => true,
        "lectern" => state
            .get_property("has_book")
            .is_some_and(|value| value == "true"),
        _ => false,
    };
    Some(has_provider)
}
pub fn java_26_2_vanilla_block_model_offset(
    block_id: &str,
    block_pos: [i32; 3],
) -> Option<[f32; 3]> {
    let block_id = normalize_java_26_2_vanilla_resource_location(block_id);
    generated_java_26_2_mining::java_26_2_block_model_offset(&block_id, block_pos)
}
fn java_26_2_mth_seed(x: i32, y: i32, z: i32) -> i64 {
    let mut seed = i64::from(x.wrapping_mul(3_129_871))
        ^ i64::from(z).wrapping_mul(116_129_781)
        ^ i64::from(y);
    seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}
fn java_26_2_model_seed_property<'a>(state: &'a BlockState, key: &str) -> Option<&'a str> {
    state.get_property(key).map(String::as_str).or_else(|| {
        state
            .get_block()
            .default_state()
            .get_property(key)
            .map(String::as_str)
    })
}
pub fn java_26_2_vanilla_block_model_seed(state: &BlockState, block_pos: BlockPos) -> Option<i64> {
    let block_id = normalize_java_26_2_vanilla_resource_location(&state.block.to_string());
    java_26_2_vanilla_block_model_seed_with_id(&block_id, state, block_pos)
}
pub fn java_26_2_vanilla_block_model_seed_with_id(
    block_id: &str,
    state: &BlockState,
    block_pos: BlockPos,
) -> Option<i64> {
    match generated_java_26_2_mining::java_26_2_block_model_seed_type(block_id)? {
        "STANDARD" => Some(java_26_2_mth_seed(block_pos.x, block_pos.y, block_pos.z)),
        "DOUBLE_BLOCK_LOWER" => {
            let half = java_26_2_model_seed_property(state, "half")?;
            let y = match half {
                "lower" => block_pos.y,
                "upper" => block_pos.y.checked_sub(1)?,
                _ => return None,
            };
            Some(java_26_2_mth_seed(block_pos.x, y, block_pos.z))
        }
        "BED_SOURCE" => {
            let part = java_26_2_model_seed_property(state, "part")?;
            let (x, z) = match part {
                "head" => (block_pos.x, block_pos.z),
                "foot" => match java_26_2_model_seed_property(state, "facing")? {
                    "north" => (block_pos.x, block_pos.z.checked_sub(1)?),
                    "south" => (block_pos.x, block_pos.z.checked_add(1)?),
                    "west" => (block_pos.x.checked_sub(1)?, block_pos.z),
                    "east" => (block_pos.x.checked_add(1)?, block_pos.z),
                    _ => return None,
                },
                _ => return None,
            };
            Some(java_26_2_mth_seed(x, block_pos.y, z))
        }
        _ => None,
    }
}
pub fn java_26_2_vanilla_tool_mining_speed(item_id: &str, block_id: &str) -> Option<f32> {
    let item_id = normalize_java_26_2_vanilla_resource_location(item_id);
    let block_id = normalize_java_26_2_vanilla_resource_location(block_id);
    generated_java_26_2_mining::java_26_2_tool_mining_speed(&item_id, &block_id)
}
pub fn java_26_2_vanilla_tool_correct_for_drops(item_id: &str, block_id: &str) -> Option<bool> {
    let item_id = normalize_java_26_2_vanilla_resource_location(item_id);
    let block_id = normalize_java_26_2_vanilla_resource_location(block_id);
    generated_java_26_2_mining::java_26_2_tool_correct_for_drops(&item_id, &block_id)
}
fn normalize_java_26_2_vanilla_resource_location(id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("minecraft:{id}")
    }
}
pub fn get_collision_shape(block: &Block, state: &BlockState) -> VoxelShape {
    get_collision_shape_with_context(block, state, BlockPos::ZERO, &CollisionContext::empty())
}
pub fn get_collision_shape_with_context(
    block: &Block,
    state: &BlockState,
    pos: BlockPos,
    context: &CollisionContext,
) -> VoxelShape {
    let name = block.location.path.as_str();
    if name == "decorated_pot" {
        return decorated_pot_shape();
    }
    if block.location.namespace == "minecraft" && name == "leaf_litter" {
        return VoxelShape::empty();
    }
    if block.location.namespace == "minecraft" && name == "bamboo" {
        return bamboo_stalk_collision_shape(pos);
    }
    if block.location.namespace == "minecraft" && name == "pointed_dripstone" {
        let [x, z] = collision_offset_xz(pos, 0.125);
        return pointed_dripstone_shape(state).move_by(x, 0.0, z);
    }
    let known_vanilla_state = block.location.namespace == "minecraft"
        && java_26_2_vanilla_block_mining_properties(name).is_some();
    if known_vanilla_state && name.ends_with("_slab") {
        let slab_type = state
            .get_property("type")
            .map(|s| s.as_str())
            .unwrap_or("bottom");
        return match slab_type {
            "top" => VoxelShape::from_aabb(AABB::new(0.0, 0.5, 0.0, 1.0, 1.0, 1.0)),
            "bottom" => VoxelShape::from_aabb(AABB::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0)),
            _ => VoxelShape::cube(),
        };
    }
    if known_vanilla_state && name.ends_with("_stairs") {
        return stair_collision_shape(state);
    }
    if known_vanilla_state && crate::chest::ChestFamily::from_block(&state.block).is_some() {
        return chest_collision_shape(state);
    }
    if known_vanilla_state && name == "piston_head" {
        return piston_head_collision_shape(state);
    }
    if state.id != 0
        && !matches!(
            name,
            "water" | "lava" | "bubble_column" | "powder_snow" | "scaffolding"
        )
        && let Some(shape) = generated_collision_shapes::shape_for_state_id(state.id)
    {
        return shape;
    }
    if is_air_like(name) {
        return VoxelShape::empty();
    }
    if is_fluid_like(name) {
        return liquid_block_collision_shape(context);
    }
    if name == "powder_snow" {
        return powder_snow_collision_shape(pos, context);
    }
    if name == "scaffolding" {
        return scaffolding_collision_shape(state, pos, context);
    }
    if name == "farmland" || name == "dirt_path" {
        return column_shape(16.0, 16.0, 0.0, 15.0);
    }
    if name == "snow" {
        return snow_layer_collision_shape(state);
    }
    if name == "cactus" {
        return column_shape(14.0, 14.0, 0.0, 15.0);
    }
    if is_carpet_like(name) {
        return carpet_collision_shape(state);
    }
    if is_flower_pot_like(name) {
        return flower_pot_shape();
    }
    if is_collisionless_plant_or_decoration(name) {
        return VoxelShape::empty();
    }
    if name.ends_with("_slab") {
        let slab_type = state
            .get_property("type")
            .map(|s| s.as_str())
            .unwrap_or("bottom");
        match slab_type {
            "top" => {
                return VoxelShape::from_aabb(AABB::new(0.0, 0.5, 0.0, 1.0, 1.0, 1.0));
            }
            "bottom" => {
                return VoxelShape::from_aabb(AABB::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0));
            }
            _ => return VoxelShape::cube(),
        }
    }
    if name.ends_with("_stairs") {
        return stair_collision_shape(state);
    }
    if name.ends_with("_fence_gate") {
        return fence_gate_collision_shape(state);
    }
    if name.ends_with("_fence") {
        let mut boxes = Vec::new();
        boxes.push(AABB::new(0.375, 0.0, 0.375, 0.625, 1.5, 0.625));
        if state
            .get_property("north")
            .map(|s| s == "true")
            .unwrap_or(false)
        {
            boxes.push(AABB::new(0.375, 0.0, 0.0, 0.625, 1.5, 0.375));
        }
        if state
            .get_property("south")
            .map(|s| s == "true")
            .unwrap_or(false)
        {
            boxes.push(AABB::new(0.375, 0.0, 0.625, 0.625, 1.5, 1.0));
        }
        if state
            .get_property("east")
            .map(|s| s == "true")
            .unwrap_or(false)
        {
            boxes.push(AABB::new(0.625, 0.0, 0.375, 1.0, 1.5, 0.625));
        }
        if state
            .get_property("west")
            .map(|s| s == "true")
            .unwrap_or(false)
        {
            boxes.push(AABB::new(0.0, 0.0, 0.375, 0.375, 1.5, 0.625));
        }
        return VoxelShape::from_aabbs(boxes);
    }
    if name.ends_with("_wall") {
        return wall_collision_shape(state);
    }
    if name.ends_with("_door") {
        return door_shape(state);
    }
    if name.ends_with("_trapdoor") {
        return trapdoor_shape(state);
    }
    if name.ends_with("_wall_hanging_sign") {
        return wall_hanging_sign_collision_shape(state);
    }
    if name == "pitcher_crop" {
        return pitcher_crop_collision_shape(state);
    }
    if name == "cocoa" {
        return cocoa_shape(state);
    }
    if name == "turtle_egg" {
        return turtle_egg_shape(state);
    }
    if name == "sniffer_egg" {
        return sniffer_egg_shape();
    }
    if matches!(
        name,
        "amethyst_cluster" | "large_amethyst_bud" | "medium_amethyst_bud" | "small_amethyst_bud"
    ) {
        return amethyst_cluster_shape(block, state);
    }
    if name == "candle" || name.ends_with("_candle") {
        return candle_shape(state);
    }
    if matches!(name, "repeater" | "comparator") {
        return column_shape(16.0, 16.0, 0.0, 2.0);
    }
    if name == "daylight_detector" {
        return column_shape(16.0, 16.0, 0.0, 6.0);
    }
    if name == "big_dripleaf" {
        return big_dripleaf_shape(state, true);
    }
    if name == "chorus_plant" {
        return chorus_plant_shape(state);
    }
    if name == "pointed_dripstone" {
        return pointed_dripstone_shape(state);
    }
    if name == "heavy_core" {
        return column_shape(8.0, 8.0, 0.0, 8.0);
    }
    if name == "brewing_stand" {
        return brewing_stand_shape();
    }
    if matches!(name, "anvil" | "chipped_anvil" | "damaged_anvil") {
        return anvil_shape(state);
    }
    if name == "stonecutter" {
        return column_shape(16.0, 16.0, 0.0, 9.0);
    }
    if name == "enchanting_table" {
        return column_shape(16.0, 16.0, 0.0, 12.0);
    }
    if name == "end_portal_frame" {
        return end_portal_frame_shape(state);
    }
    if name == "dragon_egg" {
        return column_shape(14.0, 14.0, 0.0, 16.0);
    }
    if name == "conduit" {
        return cube_shape(6.0, 6.0, 6.0);
    }
    if name == "bell" {
        return bell_shape(state);
    }
    if is_skull_like(name) {
        return skull_shape(block, state);
    }
    if matches!(name, "lightning_rod" | "end_rod") {
        return rod_shape(state);
    }
    if name.ends_with("_bed") {
        return bed_collision_shape(state);
    }
    if name == "grindstone" {
        return grindstone_collision_shape(state);
    }
    if name.contains("chest") {
        return chest_collision_shape(state);
    }
    if is_iron_bars_or_pane_like(name) {
        return cross_collision_shape(state, 2.0, 16.0, 2.0, 0.0, 16.0);
    }
    if is_chain_like(name) {
        return chain_collision_shape(state);
    }
    if is_lantern_like(name) {
        return lantern_collision_shape(state);
    }
    if name == "campfire" || name == "soul_campfire" {
        return column_shape(16.0, 16.0, 0.0, 7.0);
    }
    if name == "cake" {
        return cake_collision_shape(state);
    }
    if name == "candle_cake" || name.ends_with("_candle_cake") {
        return candle_cake_collision_shape();
    }
    VoxelShape::cube()
}
pub fn get_visual_shape_with_context(
    block: &Block,
    state: &BlockState,
    pos: BlockPos,
    context: &CollisionContext,
) -> VoxelShape {
    if block.location.namespace == "minecraft" {
        let name = block.location.path.as_str();
        if is_vanilla_iron_bars_or_pane(name)
            || is_vanilla_transparent_block(name)
            || name == "powder_snow"
        {
            return VoxelShape::empty();
        }
        if is_vanilla_fence(name) {
            return cross_collision_shape(state, 4.0, 16.0, 4.0, 0.0, 16.0);
        }
        if name == "snow" {
            return snow_layer_visual_shape(state);
        }
        if matches!(name, "mud" | "soul_sand") {
            return VoxelShape::cube();
        }
    }
    get_collision_shape_with_context(block, state, pos, context)
}
pub fn get_occlusion_shape(block: &Block, state: &BlockState) -> VoxelShape {
    let name = block.location.path.as_str();
    if is_no_occlusion_block(name) {
        return VoxelShape::empty();
    }
    let shape = get_collision_shape(block, state);
    if shape.is_empty() {
        return VoxelShape::empty();
    }
    if name.ends_with("_fence") || name.ends_with("_wall") {
        return VoxelShape::empty();
    }
    let mut normalized_boxes = Vec::new();
    for b in shape.to_aabbs() {
        normalized_boxes.push(AABB::new(
            b.min_x,
            b.min_y.min(1.0),
            b.min_z,
            b.max_x,
            b.max_y.min(1.0),
            b.max_z,
        ));
    }
    VoxelShape::from_aabbs(normalized_boxes)
}
fn stair_collision_shape(state: &BlockState) -> VoxelShape {
    let half = state
        .get_property("half")
        .map(|s| s.as_str())
        .unwrap_or("bottom");
    let facing = state
        .get_property("facing")
        .map(|s| s.as_str())
        .unwrap_or("north");
    let shape = state
        .get_property("shape")
        .map(|s| s.as_str())
        .unwrap_or("straight");
    let mut boxes = match shape {
        "outer_left" | "outer_right" => {
            vec![
                AABB::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0),
                AABB::new(0.0, 0.5, 0.0, 0.5, 1.0, 0.5),
            ]
        }
        "inner_left" | "inner_right" => {
            vec![
                AABB::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0),
                AABB::new(0.0, 0.5, 0.0, 1.0, 1.0, 0.5),
                AABB::new(0.5, 0.5, 0.5, 1.0, 1.0, 1.0),
            ]
        }
        _ => {
            vec![
                AABB::new(0.0, 0.0, 0.0, 1.0, 0.5, 1.0),
                AABB::new(0.0, 0.5, 0.0, 1.0, 1.0, 0.5),
            ]
        }
    };
    if half == "top" {
        boxes = invert_boxes_y(&boxes);
    }
    let direction = match shape {
        "inner_left" => counter_clockwise(facing),
        "outer_right" => clockwise(facing),
        _ => facing,
    };
    VoxelShape::from_aabbs(rotate_boxes_horizontal(
        &boxes,
        horizontal_rotation_steps(direction),
    ))
}
fn rotate_boxes_y_90(boxes: &[AABB]) -> Vec<AABB> {
    boxes
        .iter()
        .map(|b| {
            AABB::new(
                1.0 - b.max_z,
                b.min_y,
                b.min_x,
                1.0 - b.min_z,
                b.max_y,
                b.max_x,
            )
        })
        .collect()
}
fn invert_boxes_y(boxes: &[AABB]) -> Vec<AABB> {
    boxes
        .iter()
        .map(|b| {
            AABB::new(
                b.min_x,
                1.0 - b.max_y,
                b.min_z,
                b.max_x,
                1.0 - b.min_y,
                b.max_z,
            )
        })
        .collect()
}
fn clockwise(direction: &str) -> &'static str {
    match direction {
        "north" => "east",
        "east" => "south",
        "south" => "west",
        "west" => "north",
        _ => "north",
    }
}
fn counter_clockwise(direction: &str) -> &'static str {
    match direction {
        "north" => "west",
        "west" => "south",
        "south" => "east",
        "east" => "north",
        _ => "north",
    }
}
fn block_box(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> AABB {
    AABB::new(
        min_x / 16.0,
        min_y / 16.0,
        min_z / 16.0,
        max_x / 16.0,
        max_y / 16.0,
        max_z / 16.0,
    )
}
fn block_shape_box(
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
) -> VoxelShape {
    if min_x >= max_x || min_y >= max_y || min_z >= max_z {
        VoxelShape::empty()
    } else {
        VoxelShape::from_aabb(block_box(min_x, min_y, min_z, max_x, max_y, max_z))
    }
}
fn column_shape(size_x: f64, size_z: f64, min_y: f64, max_y: f64) -> VoxelShape {
    if size_x <= 0.0 || size_z <= 0.0 || min_y >= max_y {
        return VoxelShape::empty();
    }
    let half_x = size_x / 2.0;
    let half_z = size_z / 2.0;
    block_shape_box(
        8.0 - half_x,
        min_y,
        8.0 - half_z,
        8.0 + half_x,
        max_y,
        8.0 + half_z,
    )
}
fn decorated_pot_shape() -> VoxelShape {
    column_shape(14.0, 14.0, 0.0, 16.0)
}
fn bamboo_stalk_collision_shape(pos: BlockPos) -> VoxelShape {
    let [x, z] = collision_offset_xz(pos, 0.25);
    column_shape(3.0, 3.0, 0.0, 16.0).move_by(x, 0.0, z)
}
fn collision_offset_xz(pos: BlockPos, max_horizontal_offset: f32) -> [f64; 2] {
    let seed = java_26_2_mth_seed(pos.x, 0, pos.z);
    [0, 8].map(|shift| {
        ((f64::from(((seed >> shift) & 15) as f32 / 15.0_f32) - 0.5) * 0.5).clamp(
            -f64::from(max_horizontal_offset),
            f64::from(max_horizontal_offset),
        )
    })
}
fn segmentable_shape(state: &BlockState, height: f64, amount_property: &str) -> VoxelShape {
    let amount = state
        .get_property(amount_property)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(1)
        .clamp(1, 4);
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let mut boxes = Vec::with_capacity(amount);
    let mut direction = facing;
    for _ in 0..amount {
        boxes.push(match direction {
            "east" => AABB::new(0.5, 0.0, 0.0, 1.0, height / 16.0, 0.5),
            "south" => AABB::new(0.5, 0.0, 0.5, 1.0, height / 16.0, 1.0),
            "west" => AABB::new(0.0, 0.0, 0.5, 0.5, height / 16.0, 1.0),
            _ => AABB::new(0.0, 0.0, 0.0, 0.5, height / 16.0, 0.5),
        });
        direction = counter_clockwise(direction);
    }
    VoxelShape::from_aabbs(boxes)
}
fn cube_shape(size_x: f64, size_y: f64, size_z: f64) -> VoxelShape {
    let half_y = size_y / 2.0;
    column_shape(size_x, size_z, 8.0 - half_y, 8.0 + half_y)
}
fn snow_layer_collision_shape(state: &BlockState) -> VoxelShape {
    let layers = state
        .get_property("layers")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(1)
        .clamp(1, 8);
    column_shape(16.0, 16.0, 0.0, ((layers - 1) * 2) as f64)
}
fn snow_layer_visual_shape(state: &BlockState) -> VoxelShape {
    let layers = state
        .get_property("layers")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(1)
        .clamp(1, 8);
    column_shape(16.0, 16.0, 0.0, (layers * 2) as f64)
}
fn carpet_collision_shape(state: &BlockState) -> VoxelShape {
    if state.block.path == "pale_moss_carpet"
        && state
            .get_property("bottom")
            .map(|value| value == "false")
            .unwrap_or(false)
    {
        return VoxelShape::empty();
    }
    column_shape(16.0, 16.0, 0.0, 1.0)
}
fn chest_collision_shape(state: &BlockState) -> VoxelShape {
    const SINGLE: AABB = AABB {
        min_x: 1.0 / 16.0,
        min_y: 0.0,
        min_z: 1.0 / 16.0,
        max_x: 15.0 / 16.0,
        max_y: 14.0 / 16.0,
        max_z: 15.0 / 16.0,
    };
    if crate::chest::ChestFamily::from_block(&state.block)
        == Some(crate::chest::ChestFamily::EnderChest)
    {
        return VoxelShape::from_aabb(SINGLE);
    }
    let chest_type = state
        .get_property("type")
        .and_then(|value| crate::chest::ChestType::from_name(value))
        .unwrap_or(crate::chest::ChestType::Single);
    if chest_type == crate::chest::ChestType::Single {
        return VoxelShape::from_aabb(SINGLE);
    }
    let facing = state
        .get_property("facing")
        .and_then(|value| el_math::Direction::from_name(value))
        .filter(|direction| direction.axis() != el_math::Axis::Y)
        .unwrap_or(el_math::Direction::North);
    let connected_dir = crate::chest::chest_connected_direction(facing, chest_type);
    let mut half = SINGLE;
    match connected_dir {
        el_math::Direction::North => half.min_z = 0.0,
        el_math::Direction::South => half.max_z = 1.0,
        el_math::Direction::West => half.min_x = 0.0,
        el_math::Direction::East => half.max_x = 1.0,
        _ => {}
    }
    VoxelShape::from_aabb(half)
}
fn bed_collision_shape(state: &BlockState) -> VoxelShape {
    let mut boxes = vec![AABB::new(0.0, 3.0 / 16.0, 0.0, 1.0, 9.0 / 16.0, 1.0)];
    let north_west_leg = block_box(0.0, 0.0, 0.0, 3.0, 3.0, 3.0);
    boxes.push(north_west_leg);
    boxes.extend(rotate_boxes_y_90(&[north_west_leg]));
    let facing = state
        .get_property("facing")
        .map(|value| value.as_str())
        .unwrap_or("north");
    let connected = if state
        .get_property("part")
        .map(|value| value == "head")
        .unwrap_or(false)
    {
        opposite_direction(facing)
    } else {
        facing
    };
    let shape_direction = opposite_direction(connected);
    VoxelShape::from_aabbs(rotate_boxes_horizontal(
        &boxes,
        horizontal_rotation_steps(shape_direction),
    ))
}
fn grindstone_collision_shape(state: &BlockState) -> VoxelShape {
    let left_legs = vec![
        block_box(2.0, 6.0, 7.0, 4.0, 10.0, 16.0),
        block_box(2.0, 5.0, 3.0, 4.0, 11.0, 9.0),
    ];
    let mut north = vec![block_box(4.0, 2.0, 0.0, 12.0, 14.0, 12.0)];
    north.extend(left_legs.iter().copied());
    north.extend(invert_boxes_x(&left_legs));
    let face = state
        .get_property("face")
        .map(|value| value.as_str())
        .unwrap_or("wall");
    let mut attached = match face {
        "floor" => rotate_boxes_x_270(&north),
        "ceiling" => rotate_boxes_y_180(&rotate_boxes_x_90(&north)),
        _ => north,
    };
    let facing = state
        .get_property("facing")
        .map(|value| value.as_str())
        .unwrap_or("north");
    attached = rotate_boxes_horizontal(&attached, horizontal_rotation_steps(facing));
    VoxelShape::from_aabbs(attached)
}
fn rotate_boxes_horizontal(boxes: &[AABB], steps: usize) -> Vec<AABB> {
    let mut rotated = boxes.to_vec();
    for _ in 0..steps {
        rotated = rotate_boxes_y_90(&rotated);
    }
    rotated
}
fn horizontal_rotation_steps(direction: &str) -> usize {
    match direction {
        "east" => 1,
        "south" => 2,
        "west" => 3,
        _ => 0,
    }
}
fn piston_head_collision_shape(state: &BlockState) -> VoxelShape {
    let short = state
        .get_property("short")
        .map(|s| s == "true")
        .unwrap_or(false);
    let facing = state
        .get_property("facing")
        .map(|s| s.as_str())
        .unwrap_or("north");
    let rod_max_z = if short { 16.0 } else { 20.0 };
    let north = [
        block_box(0.0, 0.0, 0.0, 16.0, 16.0, 4.0),
        block_box(6.0, 6.0, 4.0, 10.0, 10.0, rod_max_z),
    ];
    let boxes = match facing {
        "up" => rotate_boxes_x_270(&north),
        "down" => rotate_boxes_x_90(&north),
        horizontal => rotate_boxes_horizontal(&north, horizontal_rotation_steps(horizontal)),
    };
    let transverse: Vec<f64> = if short {
        (0..=8).map(|i| f64::from(i) / 8.0).collect()
    } else {
        vec![0.0, 0.375, 0.625, 1.0]
    };
    let mut axial: Vec<f64> = (0..=if short { 4 } else { 5 })
        .map(|i| f64::from(i) / 4.0)
        .collect();
    if matches!(facing, "east" | "south" | "up") {
        axial = axial.into_iter().rev().map(|v| 1.0 - v).collect();
    }
    let coordinates = match facing {
        "east" | "west" => [axial, transverse.clone(), transverse],
        "up" | "down" => [transverse.clone(), axial, transverse],
        _ => [transverse.clone(), transverse, axial],
    };
    VoxelShape::from_aabbs(boxes).with_coordinates(coordinates)
}
fn opposite_direction(direction: &str) -> &'static str {
    match direction {
        "north" => "south",
        "south" => "north",
        "east" => "west",
        "west" => "east",
        _ => "north",
    }
}
fn rotate_boxes_x_90(boxes: &[AABB]) -> Vec<AABB> {
    boxes
        .iter()
        .map(|b| {
            AABB::new(
                b.min_x,
                b.min_z,
                1.0 - b.max_y,
                b.max_x,
                b.max_z,
                1.0 - b.min_y,
            )
        })
        .collect()
}
fn rotate_boxes_x_270(boxes: &[AABB]) -> Vec<AABB> {
    boxes
        .iter()
        .map(|b| {
            AABB::new(
                b.min_x,
                1.0 - b.max_z,
                b.min_y,
                b.max_x,
                1.0 - b.min_z,
                b.max_y,
            )
        })
        .collect()
}
fn rotate_boxes_y_180(boxes: &[AABB]) -> Vec<AABB> {
    boxes
        .iter()
        .map(|b| {
            AABB::new(
                1.0 - b.max_x,
                b.min_y,
                1.0 - b.max_z,
                1.0 - b.min_x,
                b.max_y,
                1.0 - b.min_z,
            )
        })
        .collect()
}
fn invert_boxes_x(boxes: &[AABB]) -> Vec<AABB> {
    boxes
        .iter()
        .map(|b| {
            AABB::new(
                1.0 - b.max_x,
                b.min_y,
                b.min_z,
                1.0 - b.min_x,
                b.max_y,
                b.max_z,
            )
        })
        .collect()
}
fn wall_collision_shape(state: &BlockState) -> VoxelShape {
    let mut boxes = Vec::new();
    let up = state
        .get_property("up")
        .map(|s| s == "true")
        .unwrap_or(true);
    if up {
        boxes.push(AABB::new(0.25, 0.0, 0.25, 0.75, 1.5, 0.75));
    }
    let north = state
        .get_property("north")
        .map(|s| s != "none")
        .unwrap_or(false);
    let south = state
        .get_property("south")
        .map(|s| s != "none")
        .unwrap_or(false);
    let east = state
        .get_property("east")
        .map(|s| s != "none")
        .unwrap_or(false);
    let west = state
        .get_property("west")
        .map(|s| s != "none")
        .unwrap_or(false);
    if north {
        boxes.push(AABB::new(0.3125, 0.0, 0.0, 0.6875, 1.5, 0.6875));
    }
    if south {
        boxes.push(AABB::new(0.3125, 0.0, 0.3125, 0.6875, 1.5, 1.0));
    }
    if east {
        boxes.push(AABB::new(0.3125, 0.0, 0.3125, 1.0, 1.5, 0.6875));
    }
    if west {
        boxes.push(AABB::new(0.0, 0.0, 0.3125, 0.6875, 1.5, 0.6875));
    }
    VoxelShape::from_aabbs(boxes)
}
fn fence_gate_collision_shape(state: &BlockState) -> VoxelShape {
    if state
        .get_property("open")
        .map(|value| value == "true")
        .unwrap_or(false)
    {
        return VoxelShape::empty();
    }
    let facing = state
        .get_property("facing")
        .map(|value| value.as_str())
        .unwrap_or("north");
    if matches!(facing, "east" | "west") {
        block_shape_box(6.0, 0.0, 0.0, 10.0, 24.0, 16.0)
    } else {
        block_shape_box(0.0, 0.0, 6.0, 16.0, 24.0, 10.0)
    }
}
fn cross_collision_shape(
    state: &BlockState,
    post_width: f64,
    post_height: f64,
    wall_width: f64,
    wall_bottom: f64,
    wall_top: f64,
) -> VoxelShape {
    let mut boxes = column_shape(post_width, post_width, 0.0, post_height)
        .to_aabbs()
        .to_vec();
    if state
        .get_property("north")
        .map(|value| value == "true")
        .unwrap_or(false)
    {
        boxes.push(block_box(
            8.0 - wall_width / 2.0,
            wall_bottom,
            0.0,
            8.0 + wall_width / 2.0,
            wall_top,
            8.0,
        ));
    }
    if state
        .get_property("south")
        .map(|value| value == "true")
        .unwrap_or(false)
    {
        boxes.push(block_box(
            8.0 - wall_width / 2.0,
            wall_bottom,
            8.0,
            8.0 + wall_width / 2.0,
            wall_top,
            16.0,
        ));
    }
    if state
        .get_property("east")
        .map(|value| value == "true")
        .unwrap_or(false)
    {
        boxes.push(block_box(
            8.0,
            wall_bottom,
            8.0 - wall_width / 2.0,
            16.0,
            wall_top,
            8.0 + wall_width / 2.0,
        ));
    }
    if state
        .get_property("west")
        .map(|value| value == "true")
        .unwrap_or(false)
    {
        boxes.push(block_box(
            0.0,
            wall_bottom,
            8.0 - wall_width / 2.0,
            8.0,
            wall_top,
            8.0 + wall_width / 2.0,
        ));
    }
    VoxelShape::from_aabbs(boxes)
}
fn chain_collision_shape(state: &BlockState) -> VoxelShape {
    match state
        .get_property("axis")
        .map(|value| value.as_str())
        .unwrap_or("y")
    {
        "x" => cube_shape(16.0, 3.0, 3.0),
        "z" => cube_shape(3.0, 3.0, 16.0),
        _ => cube_shape(3.0, 16.0, 3.0),
    }
}
fn lantern_collision_shape(state: &BlockState) -> VoxelShape {
    let hanging = state
        .get_property("hanging")
        .map(|value| value == "true")
        .unwrap_or(false);
    let dy = if hanging { 1.0 / 16.0 } else { 0.0 };
    let mut boxes = Vec::new();
    boxes.extend(
        column_shape(4.0, 4.0, 7.0, 9.0)
            .move_by(0.0, dy, 0.0)
            .to_aabbs()
            .iter()
            .copied(),
    );
    boxes.extend(
        column_shape(6.0, 6.0, 0.0, 7.0)
            .move_by(0.0, dy, 0.0)
            .to_aabbs()
            .iter()
            .copied(),
    );
    VoxelShape::from_aabbs(boxes)
}
fn cake_collision_shape(state: &BlockState) -> VoxelShape {
    let bites = state
        .get_property("bites")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0)
        .clamp(0, 6);
    block_shape_box(1.0 + (bites * 2) as f64, 0.0, 1.0, 15.0, 8.0, 15.0)
}
fn candle_cake_collision_shape() -> VoxelShape {
    let mut boxes = Vec::new();
    boxes.extend(column_shape(2.0, 2.0, 8.0, 14.0).to_aabbs().iter().copied());
    boxes.extend(
        column_shape(14.0, 14.0, 0.0, 8.0)
            .to_aabbs()
            .iter()
            .copied(),
    );
    VoxelShape::from_aabbs(boxes)
}
fn door_shape(state: &BlockState) -> VoxelShape {
    let open = state
        .get_property("open")
        .map(|s| s == "true")
        .unwrap_or(false);
    let facing = state
        .get_property("facing")
        .map(|s| s.as_str())
        .unwrap_or("north");
    let hinge = state
        .get_property("hinge")
        .map(|s| s.as_str())
        .unwrap_or("left");
    let t = 3.0 / 16.0;
    let mut actual_facing = facing;
    if open {
        actual_facing = match (facing, hinge) {
            ("north", "left") => "west",
            ("north", "right") => "east",
            ("south", "left") => "east",
            ("south", "right") => "west",
            ("east", "left") => "north",
            ("east", "right") => "south",
            ("west", "left") => "south",
            ("west", "right") => "north",
            _ => facing,
        };
    }
    let aabb = match actual_facing {
        "north" => AABB::new(0.0, 0.0, 1.0 - t, 1.0, 1.0, 1.0),
        "south" => AABB::new(0.0, 0.0, 0.0, 1.0, 1.0, t),
        "west" => AABB::new(1.0 - t, 0.0, 0.0, 1.0, 1.0, 1.0),
        "east" => AABB::new(0.0, 0.0, 0.0, t, 1.0, 1.0),
        _ => AABB::new(0.0, 0.0, 1.0 - t, 1.0, 1.0, 1.0),
    };
    VoxelShape::from_aabb(aabb)
}
fn trapdoor_shape(state: &BlockState) -> VoxelShape {
    let open = state
        .get_property("open")
        .map(|s| s == "true")
        .unwrap_or(false);
    let half = state
        .get_property("half")
        .map(|s| s.as_str())
        .unwrap_or("bottom");
    let facing = state
        .get_property("facing")
        .map(|s| s.as_str())
        .unwrap_or("north");
    let t = 3.0 / 16.0;
    if open {
        let aabb = match facing {
            "north" => AABB::new(0.0, 0.0, 1.0 - t, 1.0, 1.0, 1.0),
            "south" => AABB::new(0.0, 0.0, 0.0, 1.0, 1.0, t),
            "west" => AABB::new(1.0 - t, 0.0, 0.0, 1.0, 1.0, 1.0),
            "east" => AABB::new(0.0, 0.0, 0.0, t, 1.0, 1.0),
            _ => AABB::new(0.0, 0.0, 1.0 - t, 1.0, 1.0, 1.0),
        };
        VoxelShape::from_aabb(aabb)
    } else {
        let aabb = match half {
            "top" => AABB::new(0.0, 1.0 - t, 0.0, 1.0, 1.0, 1.0),
            _ => AABB::new(0.0, 0.0, 0.0, 1.0, t, 1.0),
        };
        VoxelShape::from_aabb(aabb)
    }
}
fn ladder_shape(state: &BlockState) -> VoxelShape {
    let facing = state
        .get_property("facing")
        .map(|s| s.as_str())
        .unwrap_or("north");
    let t = 3.0 / 16.0;
    let aabb = match facing {
        "north" => AABB::new(0.0, 0.0, 1.0 - t, 1.0, 1.0, 1.0),
        "south" => AABB::new(0.0, 0.0, 0.0, 1.0, 1.0, t),
        "west" => AABB::new(1.0 - t, 0.0, 0.0, 1.0, 1.0, 1.0),
        "east" => AABB::new(0.0, 0.0, 0.0, t, 1.0, 1.0),
        _ => AABB::new(0.0, 0.0, 1.0 - t, 1.0, 1.0, 1.0),
    };
    VoxelShape::from_aabb(aabb)
}
fn wall_outline_shape(state: &BlockState) -> VoxelShape {
    let mut boxes = Vec::new();
    let up = state
        .get_property("up")
        .map(|s| s == "true")
        .unwrap_or(true);
    if up {
        boxes.push(AABB::new(0.25, 0.0, 0.25, 0.75, 1.0, 0.75));
    }
    let sides = [
        (
            "north",
            state
                .get_property("north")
                .map(String::as_str)
                .unwrap_or("none"),
        ),
        (
            "east",
            state
                .get_property("east")
                .map(String::as_str)
                .unwrap_or("none"),
        ),
        (
            "south",
            state
                .get_property("south")
                .map(String::as_str)
                .unwrap_or("none"),
        ),
        (
            "west",
            state
                .get_property("west")
                .map(String::as_str)
                .unwrap_or("none"),
        ),
    ];
    for (dir, wall_side) in sides {
        if wall_side == "none" {
            continue;
        }
        let height = if wall_side == "tall" {
            1.0
        } else {
            14.0 / 16.0
        };
        let aabb = match dir {
            "north" => AABB::new(0.3125, 0.0, 0.0, 0.6875, height, 0.6875),
            "east" => AABB::new(0.3125, 0.0, 0.3125, 1.0, height, 0.6875),
            "south" => AABB::new(0.3125, 0.0, 0.3125, 0.6875, height, 1.0),
            "west" => AABB::new(0.0, 0.0, 0.3125, 0.6875, height, 0.6875),
            _ => continue,
        };
        boxes.push(aabb);
    }
    VoxelShape::from_aabbs(boxes)
}
fn fence_gate_outline_shape(state: &BlockState) -> VoxelShape {
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let in_wall = state
        .get_property("in_wall")
        .map(|s| s == "true")
        .unwrap_or(false);
    let height = if in_wall { 13.0 / 16.0 } else { 1.0 };
    if matches!(facing, "east" | "west") {
        VoxelShape::from_aabb(AABB::new(0.375, 0.0, 0.0, 0.625, height, 1.0))
    } else {
        VoxelShape::from_aabb(AABB::new(0.0, 0.0, 0.375, 1.0, height, 0.625))
    }
}
fn fence_outline_shape(state: &BlockState) -> VoxelShape {
    let mut boxes = Vec::new();
    boxes.push(AABB::new(0.375, 0.0, 0.375, 0.625, 1.0, 0.625));
    if state
        .get_property("north")
        .map(|s| s == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(0.375, 0.0, 0.0, 0.625, 1.0, 0.5));
    }
    if state
        .get_property("south")
        .map(|s| s == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(0.375, 0.0, 0.5, 0.625, 1.0, 1.0));
    }
    if state
        .get_property("east")
        .map(|s| s == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(0.5, 0.0, 0.375, 1.0, 1.0, 0.625));
    }
    if state
        .get_property("west")
        .map(|s| s == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(0.0, 0.0, 0.375, 0.5, 1.0, 0.625));
    }
    VoxelShape::from_aabbs(boxes)
}
fn lever_shape(state: &BlockState) -> VoxelShape {
    let px = 1.0 / 16.0;
    let face = state
        .get_property("face")
        .map(String::as_str)
        .unwrap_or("wall");
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let aabb = match face {
        "floor" => match facing {
            "east" | "west" => AABB::new(4.0 * px, 0.0, 5.0 * px, 12.0 * px, 6.0 * px, 11.0 * px),
            _ => AABB::new(5.0 * px, 0.0, 4.0 * px, 11.0 * px, 6.0 * px, 12.0 * px),
        },
        "ceiling" => match facing {
            "east" | "west" => AABB::new(4.0 * px, 10.0 * px, 5.0 * px, 12.0 * px, 1.0, 11.0 * px),
            _ => AABB::new(5.0 * px, 10.0 * px, 4.0 * px, 11.0 * px, 1.0, 12.0 * px),
        },
        _ => match facing {
            "south" => AABB::new(5.0 * px, 4.0 * px, 0.0, 11.0 * px, 12.0 * px, 6.0 * px),
            "west" => AABB::new(10.0 * px, 4.0 * px, 5.0 * px, 1.0, 12.0 * px, 11.0 * px),
            "east" => AABB::new(0.0, 4.0 * px, 5.0 * px, 6.0 * px, 12.0 * px, 11.0 * px),
            _ => AABB::new(5.0 * px, 4.0 * px, 10.0 * px, 11.0 * px, 12.0 * px, 1.0),
        },
    };
    VoxelShape::from_aabb(aabb)
}
fn button_shape(state: &BlockState) -> VoxelShape {
    let px = 1.0 / 16.0;
    let face = state
        .get_property("face")
        .map(String::as_str)
        .unwrap_or("wall");
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let powered = state
        .get_property("powered")
        .map(|s| s == "true")
        .unwrap_or(false);
    let depth = if powered { 1.0 * px } else { 2.0 * px };
    let aabb = match face {
        "floor" => match facing {
            "east" | "west" => AABB::new(6.0 * px, 0.0, 5.0 * px, 10.0 * px, depth, 11.0 * px),
            _ => AABB::new(5.0 * px, 0.0, 6.0 * px, 11.0 * px, depth, 10.0 * px),
        },
        "ceiling" => match facing {
            "east" | "west" => {
                AABB::new(6.0 * px, 1.0 - depth, 5.0 * px, 10.0 * px, 1.0, 11.0 * px)
            }
            _ => AABB::new(5.0 * px, 1.0 - depth, 6.0 * px, 11.0 * px, 1.0, 10.0 * px),
        },
        _ => match facing {
            "south" => AABB::new(5.0 * px, 6.0 * px, 0.0, 11.0 * px, 10.0 * px, depth),
            "west" => AABB::new(1.0 - depth, 6.0 * px, 5.0 * px, 1.0, 10.0 * px, 11.0 * px),
            "east" => AABB::new(0.0, 6.0 * px, 5.0 * px, depth, 10.0 * px, 11.0 * px),
            _ => AABB::new(5.0 * px, 6.0 * px, 1.0 - depth, 11.0 * px, 10.0 * px, 1.0),
        },
    };
    VoxelShape::from_aabb(aabb)
}
fn pressure_plate_shape(state: &BlockState) -> VoxelShape {
    let powered = state
        .get_property("powered")
        .map(|s| s == "true")
        .unwrap_or(false)
        || state
            .get_property("power")
            .and_then(|v| v.parse::<i32>().ok())
            .map(|p| p > 0)
            .unwrap_or(false);
    let height = if powered { 0.5 } else { 1.0 };
    column_shape(14.0, 14.0, 0.0, height)
}
fn redstone_wire_shape(state: &BlockState) -> VoxelShape {
    let px = 1.0 / 16.0;
    let mut boxes = vec![AABB::new(
        3.0 * px,
        0.0,
        3.0 * px,
        13.0 * px,
        1.0 * px,
        13.0 * px,
    )];
    let north = state
        .get_property("north")
        .map(String::as_str)
        .unwrap_or("none");
    let east = state
        .get_property("east")
        .map(String::as_str)
        .unwrap_or("none");
    let south = state
        .get_property("south")
        .map(String::as_str)
        .unwrap_or("none");
    let west = state
        .get_property("west")
        .map(String::as_str)
        .unwrap_or("none");
    if north == "side" || north == "up" {
        boxes.push(AABB::new(3.0 * px, 0.0, 0.0, 13.0 * px, 1.0 * px, 8.0 * px));
    }
    if north == "up" {
        boxes.push(AABB::new(3.0 * px, 0.0, 0.0, 13.0 * px, 1.0, 1.0 * px));
    }
    if east == "side" || east == "up" {
        boxes.push(AABB::new(8.0 * px, 0.0, 3.0 * px, 1.0, 1.0 * px, 13.0 * px));
    }
    if east == "up" {
        boxes.push(AABB::new(15.0 * px, 0.0, 3.0 * px, 1.0, 1.0, 13.0 * px));
    }
    if south == "side" || south == "up" {
        boxes.push(AABB::new(3.0 * px, 0.0, 8.0 * px, 13.0 * px, 1.0 * px, 1.0));
    }
    if south == "up" {
        boxes.push(AABB::new(3.0 * px, 0.0, 15.0 * px, 13.0 * px, 1.0, 1.0));
    }
    if west == "side" || west == "up" {
        boxes.push(AABB::new(0.0, 0.0, 3.0 * px, 8.0 * px, 1.0 * px, 13.0 * px));
    }
    if west == "up" {
        boxes.push(AABB::new(0.0, 0.0, 3.0 * px, 1.0 * px, 1.0, 13.0 * px));
    }
    VoxelShape::from_aabbs(boxes)
}
fn rod_shape(state: &BlockState) -> VoxelShape {
    let px = 1.0 / 16.0;
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("up");
    let aabb = match facing {
        "north" | "south" => AABB::new(6.0 * px, 6.0 * px, 0.0, 10.0 * px, 10.0 * px, 1.0),
        "east" | "west" => AABB::new(0.0, 6.0 * px, 6.0 * px, 1.0, 10.0 * px, 10.0 * px),
        _ => AABB::new(6.0 * px, 0.0, 6.0 * px, 10.0 * px, 1.0, 10.0 * px),
    };
    VoxelShape::from_aabb(aabb)
}
fn lectern_outline_shape(state: &BlockState) -> VoxelShape {
    let px = 1.0 / 16.0;
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let mut boxes = vec![
        AABB::new(0.0, 0.0, 0.0, 1.0, 2.0 * px, 1.0),
        AABB::new(
            4.0 * px,
            2.0 * px,
            4.0 * px,
            12.0 * px,
            14.0 * px,
            12.0 * px,
        ),
    ];
    let desk = match facing {
        "south" => {
            vec![
                AABB::new(0.0, 10.0 * px, 10.666667 * px, 1.0, 14.0 * px, 15.0 * px),
                AABB::new(0.0, 12.0 * px, 6.333333 * px, 1.0, 1.0, 10.666667 * px),
                AABB::new(0.0, 14.0 * px, 2.0 * px, 1.0, 18.0 * px, 6.333333 * px),
            ]
        }
        "west" => {
            vec![
                AABB::new(1.0 * px, 10.0 * px, 0.0, 5.333333 * px, 14.0 * px, 1.0),
                AABB::new(5.333333 * px, 12.0 * px, 0.0, 9.666667 * px, 1.0, 1.0),
                AABB::new(9.666667 * px, 14.0 * px, 0.0, 14.0 * px, 18.0 * px, 1.0),
            ]
        }
        "east" => {
            vec![
                AABB::new(10.666667 * px, 10.0 * px, 0.0, 15.0 * px, 14.0 * px, 1.0),
                AABB::new(6.333333 * px, 12.0 * px, 0.0, 10.666667 * px, 1.0, 1.0),
                AABB::new(2.0 * px, 14.0 * px, 0.0, 6.333333 * px, 18.0 * px, 1.0),
            ]
        }
        _ => {
            vec![
                AABB::new(0.0, 10.0 * px, 1.0 * px, 1.0, 14.0 * px, 5.333333 * px),
                AABB::new(0.0, 12.0 * px, 5.333333 * px, 1.0, 1.0, 9.666667 * px),
                AABB::new(0.0, 14.0 * px, 9.666667 * px, 1.0, 18.0 * px, 14.0 * px),
            ]
        }
    };
    boxes.extend(desk);
    VoxelShape::from_aabbs(boxes)
}
fn bell_shape(state: &BlockState) -> VoxelShape {
    let px = 1.0 / 16.0;
    let attachment = state
        .get_property("attachment")
        .map(String::as_str)
        .unwrap_or("floor");
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    if attachment == "floor" {
        let aabb = match facing {
            "east" | "west" => AABB::new(4.0 * px, 0.0, 0.0, 12.0 * px, 1.0, 1.0),
            _ => AABB::new(0.0, 0.0, 4.0 * px, 1.0, 1.0, 12.0 * px),
        };
        return VoxelShape::from_aabb(aabb);
    }
    let mut boxes = vec![
        AABB::new(
            5.0 * px,
            6.0 * px,
            5.0 * px,
            11.0 * px,
            13.0 * px,
            11.0 * px,
        ),
        AABB::new(4.0 * px, 4.0 * px, 4.0 * px, 12.0 * px, 6.0 * px, 12.0 * px),
    ];
    match attachment {
        "ceiling" => {
            boxes.push(AABB::new(
                7.0 * px,
                13.0 * px,
                7.0 * px,
                9.0 * px,
                1.0,
                9.0 * px,
            ));
        }
        "double_wall" => {
            let bar = match facing {
                "east" | "west" => AABB::new(0.0, 13.0 * px, 7.0 * px, 1.0, 15.0 * px, 9.0 * px),
                _ => AABB::new(7.0 * px, 13.0 * px, 0.0, 9.0 * px, 15.0 * px, 1.0),
            };
            boxes.push(bar);
        }
        "single_wall" => {
            let bracket = match facing {
                "south" => AABB::new(7.0 * px, 13.0 * px, 3.0 * px, 9.0 * px, 15.0 * px, 1.0),
                "west" => AABB::new(0.0, 13.0 * px, 7.0 * px, 13.0 * px, 15.0 * px, 9.0 * px),
                "east" => AABB::new(3.0 * px, 13.0 * px, 7.0 * px, 1.0, 15.0 * px, 9.0 * px),
                _ => AABB::new(7.0 * px, 13.0 * px, 0.0, 9.0 * px, 15.0 * px, 13.0 * px),
            };
            boxes.push(bracket);
        }
        _ => {}
    }
    VoxelShape::from_aabbs(boxes)
}
fn skull_shape(block: &Block, state: &BlockState) -> VoxelShape {
    let name = block.location.path.as_str();
    let is_piglin = name.contains("piglin");
    if name.contains("wall") {
        let facing = state
            .get_property("facing")
            .map(String::as_str)
            .unwrap_or("north");
        let base_aabb = if is_piglin {
            block_box(3.0, 4.0, 8.0, 13.0, 12.0, 16.0)
        } else {
            block_box(4.0, 4.0, 8.0, 12.0, 12.0, 16.0)
        };
        let rotated = rotate_boxes_horizontal(&[base_aabb], horizontal_rotation_steps(facing));
        VoxelShape::from_aabbs(rotated)
    } else if is_piglin {
        column_shape(10.0, 10.0, 0.0, 8.0)
    } else {
        column_shape(8.0, 8.0, 0.0, 8.0)
    }
}
fn banner_shape(block: &Block, state: &BlockState) -> VoxelShape {
    let name = block.location.path.as_str();
    if name.contains("wall") {
        let facing = state
            .get_property("facing")
            .map(String::as_str)
            .unwrap_or("north");
        let base_aabb = block_box(0.0, 0.0, 14.0, 16.0, 12.5, 16.0);
        let rotated = rotate_boxes_horizontal(&[base_aabb], horizontal_rotation_steps(facing));
        VoxelShape::from_aabbs(rotated)
    } else {
        column_shape(8.0, 8.0, 0.0, 16.0)
    }
}
fn sign_outline_shape(block: &Block, state: &BlockState) -> VoxelShape {
    let name = block.location.path.as_str();
    if name.ends_with("_wall_hanging_sign") {
        wall_hanging_sign_outline_shape(state)
    } else if name.ends_with("_hanging_sign") {
        ceiling_hanging_sign_outline_shape(state)
    } else if name.ends_with("_wall_sign") {
        let facing = state
            .get_property("facing")
            .map(String::as_str)
            .unwrap_or("north");
        let base_aabb = block_box(0.0, 4.5, 14.0, 16.0, 12.5, 16.0);
        let rotated = rotate_boxes_horizontal(&[base_aabb], horizontal_rotation_steps(facing));
        VoxelShape::from_aabbs(rotated)
    } else {
        column_shape(8.0, 8.0, 0.0, 16.0)
    }
}
fn ceiling_hanging_sign_outline_shape(state: &BlockState) -> VoxelShape {
    let rotation = state
        .get_property("rotation")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0);
    match rotation {
        0 | 8 => block_shape_box(1.0, 0.0, 7.0, 15.0, 10.0, 9.0),
        4 | 12 => block_shape_box(7.0, 0.0, 1.0, 9.0, 10.0, 15.0),
        _ => column_shape(10.0, 10.0, 0.0, 16.0),
    }
}
fn wall_hanging_sign_plank_box(state: &BlockState) -> AABB {
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    match facing {
        "east" | "west" => block_box(6.0, 14.0, 0.0, 10.0, 16.0, 16.0),
        _ => block_box(0.0, 14.0, 6.0, 16.0, 16.0, 10.0),
    }
}
fn wall_hanging_sign_outline_shape(state: &BlockState) -> VoxelShape {
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let plank = wall_hanging_sign_plank_box(state);
    let board = match facing {
        "east" | "west" => block_box(7.0, 0.0, 1.0, 9.0, 10.0, 15.0),
        _ => block_box(1.0, 0.0, 7.0, 15.0, 10.0, 9.0),
    };
    VoxelShape::from_aabbs(vec![plank, board])
}
fn wall_hanging_sign_collision_shape(state: &BlockState) -> VoxelShape {
    VoxelShape::from_aabb(wall_hanging_sign_plank_box(state))
}
fn pointed_dripstone_shape(state: &BlockState) -> VoxelShape {
    let thickness = state
        .get_property("thickness")
        .map(String::as_str)
        .unwrap_or("tip");
    let direction = state
        .get_property("vertical_direction")
        .map(String::as_str)
        .unwrap_or("up");
    match thickness {
        "tip_merge" => column_shape(6.0, 6.0, 0.0, 16.0),
        "tip" => {
            if direction == "down" {
                column_shape(6.0, 6.0, 5.0, 16.0)
            } else {
                column_shape(6.0, 6.0, 0.0, 11.0)
            }
        }
        "frustum" => column_shape(8.0, 8.0, 0.0, 16.0),
        "middle" => column_shape(10.0, 10.0, 0.0, 16.0),
        "base" => column_shape(12.0, 12.0, 0.0, 16.0),
        _ => column_shape(6.0, 6.0, 0.0, 11.0),
    }
}
fn amethyst_cluster_shape(block: &Block, state: &BlockState) -> VoxelShape {
    let name = block.location.path.as_str();
    let (height, width): (f64, f64) = if name == "amethyst_cluster" {
        (7.0, 10.0)
    } else if name == "large_amethyst_bud" {
        (5.0, 10.0)
    } else if name == "medium_amethyst_bud" {
        (4.0, 10.0)
    } else if name == "small_amethyst_bud" {
        (3.0, 8.0)
    } else {
        return VoxelShape::cube();
    };
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("up");
    let half_w = width / 2.0;
    let min_w = 8.0 - half_w;
    let max_w = 8.0 + half_w;
    let aabb = match facing {
        "up" => block_box(min_w, 0.0, min_w, max_w, height, max_w),
        "down" => block_box(min_w, 16.0 - height, min_w, max_w, 16.0, max_w),
        "north" => block_box(min_w, min_w, 16.0 - height, max_w, max_w, 16.0),
        "south" => block_box(min_w, min_w, 0.0, max_w, max_w, height),
        "east" => block_box(0.0, min_w, min_w, height, max_w, max_w),
        "west" => block_box(16.0 - height, min_w, min_w, 16.0, max_w, max_w),
        _ => block_box(min_w, 0.0, min_w, max_w, height, max_w),
    };
    VoxelShape::from_aabb(aabb)
}
fn candle_shape(state: &BlockState) -> VoxelShape {
    let candles = state
        .get_property("candles")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1);
    match candles {
        1 => column_shape(2.0, 2.0, 0.0, 6.0),
        2 => block_shape_box(5.0, 0.0, 6.0, 11.0, 6.0, 9.0),
        3 => block_shape_box(5.0, 0.0, 6.0, 10.0, 6.0, 11.0),
        4 => block_shape_box(5.0, 0.0, 5.0, 11.0, 6.0, 10.0),
        _ => column_shape(2.0, 2.0, 0.0, 6.0),
    }
}
fn crop_shape(state: &BlockState, max_age: i32, base_h: i32, step_h: i32) -> VoxelShape {
    let age = state
        .get_property("age")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0)
        .clamp(0, max_age);
    let height = f64::from(base_h + age * step_h).min(16.0);
    column_shape(16.0, 16.0, 0.0, height)
}
fn torchflower_crop_shape(state: &BlockState) -> VoxelShape {
    let age = state
        .get_property("age")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0)
        .clamp(0, 1);
    let height = if age == 0 { 6.0 } else { 10.0 };
    column_shape(6.0, 6.0, 0.0, height)
}
fn pitcher_crop_outline_shape(state: &BlockState) -> VoxelShape {
    let age = state
        .get_property("age")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0)
        .min(4);
    let half = state
        .get_property("half")
        .map(String::as_str)
        .unwrap_or("lower");
    const PLANT_HEIGHTS: [i32; 5] = [0, 9, 11, 22, 26];
    let height = (if age == 0 { 4 } else { 6 }) + PLANT_HEIGHTS[age];
    let width = if age == 0 { 6.0 } else { 10.0 };
    if half == "upper" {
        let max_y = (height - 1 - 16).max(0) as f64;
        if max_y <= 0.0 {
            VoxelShape::empty()
        } else {
            column_shape(width, width, 0.0, max_y)
        }
    } else {
        let max_y = (height - 1).min(16) as f64;
        column_shape(width, width, -1.0, max_y)
    }
}
fn pitcher_crop_collision_shape(state: &BlockState) -> VoxelShape {
    let half = state
        .get_property("half")
        .map(String::as_str)
        .unwrap_or("lower");
    if half == "upper" {
        return VoxelShape::empty();
    }
    let age = state
        .get_property("age")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0);
    if age == 0 {
        column_shape(6.0, 6.0, -1.0, 3.0)
    } else {
        column_shape(10.0, 10.0, -1.0, 5.0)
    }
}
fn nether_wart_shape(state: &BlockState) -> VoxelShape {
    let age = state
        .get_property("age")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0)
        .clamp(0, 3);
    let height = match age {
        0 => 5.0,
        1 => 8.0,
        2 => 11.0,
        _ => 14.0,
    };
    column_shape(16.0, 16.0, 0.0, height)
}
fn crop_stem_shape(state: &BlockState) -> VoxelShape {
    let age = state
        .get_property("age")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0)
        .clamp(0, 7);
    let height = f64::from(2 + age * 2);
    column_shape(2.0, 2.0, 0.0, height)
}
fn attached_stem_shape(state: &BlockState) -> VoxelShape {
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let base = block_box(6.0, 0.0, 0.0, 10.0, 10.0, 10.0);
    let rotated = rotate_boxes_horizontal(&[base], horizontal_rotation_steps(facing));
    VoxelShape::from_aabbs(rotated)
}
fn cocoa_shape(state: &BlockState) -> VoxelShape {
    let age = state
        .get_property("age")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0)
        .clamp(0, 2);
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let base_box = match age {
        0 => block_box(6.0, 7.0, 1.0, 10.0, 12.0, 5.0),
        1 => block_box(5.0, 5.0, 1.0, 11.0, 12.0, 7.0),
        _ => block_box(4.0, 3.0, 1.0, 12.0, 12.0, 9.0),
    };
    let rotated = rotate_boxes_horizontal(&[base_box], horizontal_rotation_steps(facing));
    VoxelShape::from_aabbs(rotated)
}
fn sweet_berry_bush_shape(state: &BlockState) -> VoxelShape {
    let age = state
        .get_property("age")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(0)
        .clamp(0, 3);
    match age {
        0 => column_shape(10.0, 10.0, 0.0, 8.0),
        1 | 2 => column_shape(14.0, 14.0, 0.0, 16.0),
        _ => VoxelShape::cube(),
    }
}
fn big_dripleaf_shape(state: &BlockState, collision_only: bool) -> VoxelShape {
    let tilt = state
        .get_property("tilt")
        .map(String::as_str)
        .unwrap_or("none");
    let leaf = match tilt {
        "none" | "unstable" => Some(column_shape(16.0, 16.0, 11.0, 15.0)),
        "partial" => Some(column_shape(16.0, 16.0, 11.0, 13.0)),
        _ => None,
    };
    if collision_only {
        return leaf.unwrap_or_else(VoxelShape::empty);
    }
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let base_stem = block_box(5.0, 0.0, 9.0, 11.0, 13.0, 15.0);
    let stem = rotate_boxes_horizontal(&[base_stem], horizontal_rotation_steps(facing));
    let mut boxes = stem;
    if let Some(leaf_shape) = leaf {
        boxes.extend_from_slice(leaf_shape.to_aabbs());
    }
    VoxelShape::from_aabbs(boxes)
}
fn big_dripleaf_stem_shape(state: &BlockState) -> VoxelShape {
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let base_stem = block_box(5.0, 0.0, 9.0, 11.0, 16.0, 15.0);
    let stem = rotate_boxes_horizontal(&[base_stem], horizontal_rotation_steps(facing));
    VoxelShape::from_aabbs(stem)
}
fn multiface_shape(state: &BlockState) -> VoxelShape {
    let mut boxes = Vec::new();
    let px = 1.0 / 16.0;
    if state
        .get_property("down")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(0.0, 0.0, 0.0, 1.0, px, 1.0));
    }
    if state
        .get_property("up")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(0.0, 1.0 - px, 0.0, 1.0, 1.0, 1.0));
    }
    if state
        .get_property("north")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(0.0, 0.0, 0.0, 1.0, 1.0, px));
    }
    if state
        .get_property("south")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(0.0, 0.0, 1.0 - px, 1.0, 1.0, 1.0));
    }
    if state
        .get_property("west")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(0.0, 0.0, 0.0, px, 1.0, 1.0));
    }
    if state
        .get_property("east")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(1.0 - px, 0.0, 0.0, 1.0, 1.0, 1.0));
    }
    if boxes.is_empty() {
        VoxelShape::cube()
    } else {
        VoxelShape::from_aabbs(boxes)
    }
}
fn chorus_plant_shape(state: &BlockState) -> VoxelShape {
    let px = 1.0 / 16.0;
    let mut boxes = vec![AABB::new(
        3.0 * px,
        3.0 * px,
        3.0 * px,
        13.0 * px,
        13.0 * px,
        13.0 * px,
    )];
    if state
        .get_property("down")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(
            3.0 * px,
            0.0,
            3.0 * px,
            13.0 * px,
            3.0 * px,
            13.0 * px,
        ));
    }
    if state
        .get_property("up")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(
            3.0 * px,
            13.0 * px,
            3.0 * px,
            13.0 * px,
            1.0,
            13.0 * px,
        ));
    }
    if state
        .get_property("north")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(
            3.0 * px,
            3.0 * px,
            0.0,
            13.0 * px,
            13.0 * px,
            3.0 * px,
        ));
    }
    if state
        .get_property("south")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(
            3.0 * px,
            3.0 * px,
            13.0 * px,
            13.0 * px,
            13.0 * px,
            1.0,
        ));
    }
    if state
        .get_property("west")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(
            0.0,
            3.0 * px,
            3.0 * px,
            3.0 * px,
            13.0 * px,
            13.0 * px,
        ));
    }
    if state
        .get_property("east")
        .map(|v| v == "true")
        .unwrap_or(false)
    {
        boxes.push(AABB::new(
            13.0 * px,
            3.0 * px,
            3.0 * px,
            1.0,
            13.0 * px,
            13.0 * px,
        ));
    }
    VoxelShape::from_aabbs(boxes)
}
fn turtle_egg_shape(state: &BlockState) -> VoxelShape {
    let eggs = state
        .get_property("eggs")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1);
    if eggs <= 1 {
        block_shape_box(3.0, 0.0, 3.0, 12.0, 7.0, 12.0)
    } else {
        column_shape(14.0, 14.0, 0.0, 7.0)
    }
}
fn sniffer_egg_shape() -> VoxelShape {
    block_shape_box(1.0, 0.0, 2.0, 15.0, 16.0, 14.0)
}
fn sea_pickle_shape(state: &BlockState) -> VoxelShape {
    let pickles = state
        .get_property("pickles")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1);
    match pickles {
        2 => column_shape(10.0, 10.0, 0.0, 6.0),
        3 => column_shape(12.0, 12.0, 0.0, 6.0),
        4 => column_shape(12.0, 12.0, 0.0, 7.0),
        _ => column_shape(4.0, 4.0, 0.0, 6.0),
    }
}
fn pale_moss_carpet_outline_shape(state: &BlockState) -> VoxelShape {
    let px = 1.0 / 16.0;
    let mut boxes = Vec::new();
    if state
        .get_property("bottom")
        .map(|v| v == "true")
        .unwrap_or(true)
    {
        boxes.push(AABB::new(0.0, 0.0, 0.0, 1.0, px, 1.0));
    }
    for (prop, dir) in [
        ("north", "north"),
        ("east", "east"),
        ("south", "south"),
        ("west", "west"),
    ] {
        let side = state
            .get_property(prop)
            .map(String::as_str)
            .unwrap_or("none");
        let height = match side {
            "low" => 10.0 * px,
            "tall" => 1.0,
            _ => continue,
        };
        let side_box = match dir {
            "north" => AABB::new(0.0, 0.0, 0.0, 1.0, height, px),
            "south" => AABB::new(0.0, 0.0, 1.0 - px, 1.0, height, 1.0),
            "west" => AABB::new(0.0, 0.0, 0.0, px, height, 1.0),
            "east" => AABB::new(1.0 - px, 0.0, 0.0, 1.0, height, 1.0),
            _ => continue,
        };
        boxes.push(side_box);
    }
    if boxes.is_empty() {
        VoxelShape::cube()
    } else {
        VoxelShape::from_aabbs(boxes)
    }
}
fn pale_hanging_moss_shape(state: &BlockState) -> VoxelShape {
    let tip = state
        .get_property("tip")
        .map(|v| v == "true")
        .unwrap_or(false);
    if tip {
        column_shape(14.0, 14.0, 2.0, 16.0)
    } else {
        column_shape(14.0, 14.0, 0.0, 16.0)
    }
}
fn snow_outline_shape(state: &BlockState) -> VoxelShape {
    let layers = state
        .get_property("layers")
        .and_then(|v| v.parse::<i32>().ok())
        .unwrap_or(1)
        .clamp(1, 8);
    column_shape(16.0, 16.0, 0.0, (layers * 2) as f64)
}
fn azalea_shape() -> VoxelShape {
    let mut boxes = Vec::new();
    boxes.extend_from_slice(column_shape(16.0, 16.0, 8.0, 16.0).to_aabbs());
    boxes.extend_from_slice(column_shape(4.0, 4.0, 0.0, 8.0).to_aabbs());
    VoxelShape::from_aabbs(boxes)
}
fn mangrove_propagule_shape(state: &BlockState) -> VoxelShape {
    let hanging = state
        .get_property("hanging")
        .map(|v| v == "true")
        .unwrap_or(false);
    if hanging {
        let age = state
            .get_property("age")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(0)
            .min(4);
        const MIN_Y: [f64; 5] = [13.0, 10.0, 7.0, 3.0, 0.0];
        column_shape(2.0, 2.0, MIN_Y[age], 16.0)
    } else {
        column_shape(2.0, 2.0, 0.0, 16.0)
    }
}
fn bamboo_outline_shape(state: &BlockState) -> VoxelShape {
    let leaves = state
        .get_property("leaves")
        .map(String::as_str)
        .unwrap_or("none");
    if leaves == "large" {
        column_shape(10.0, 10.0, 0.0, 16.0)
    } else {
        column_shape(6.0, 6.0, 0.0, 16.0)
    }
}
fn brewing_stand_shape() -> VoxelShape {
    let mut boxes = Vec::new();
    boxes.extend_from_slice(column_shape(14.0, 14.0, 0.0, 2.0).to_aabbs());
    boxes.extend_from_slice(column_shape(2.0, 2.0, 2.0, 14.0).to_aabbs());
    VoxelShape::from_aabbs(boxes)
}
fn anvil_shape(state: &BlockState) -> VoxelShape {
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    if matches!(facing, "north" | "south") {
        VoxelShape::from_aabbs(vec![
            block_box(2.0, 0.0, 2.0, 14.0, 4.0, 14.0),
            block_box(4.0, 4.0, 3.0, 12.0, 5.0, 13.0),
            block_box(6.0, 5.0, 4.0, 10.0, 10.0, 12.0),
            block_box(3.0, 10.0, 0.0, 13.0, 16.0, 16.0),
        ])
    } else {
        VoxelShape::from_aabbs(vec![
            block_box(2.0, 0.0, 2.0, 14.0, 4.0, 14.0),
            block_box(3.0, 4.0, 4.0, 13.0, 5.0, 12.0),
            block_box(4.0, 5.0, 6.0, 12.0, 10.0, 10.0),
            block_box(0.0, 10.0, 3.0, 16.0, 16.0, 13.0),
        ])
    }
}
fn end_portal_frame_shape(state: &BlockState) -> VoxelShape {
    let has_eye = state
        .get_property("eye")
        .map(|v| v == "true")
        .unwrap_or(false);
    let base = column_shape(16.0, 16.0, 0.0, 13.0);
    if has_eye {
        let eye = column_shape(8.0, 8.0, 13.0, 16.0);
        let mut boxes = base.to_aabbs().to_vec();
        boxes.extend_from_slice(eye.to_aabbs());
        VoxelShape::from_aabbs(boxes)
    } else {
        base
    }
}
fn is_skull_like(name: &str) -> bool {
    name.ends_with("_head")
        || name.ends_with("_wall_head")
        || name.ends_with("_skull")
        || name.ends_with("_wall_skull")
}
pub fn has_large_collision_shape(block: &Block, state: &BlockState) -> bool {
    let name = block.location.path.as_str();
    if is_dynamic_shape_block_like_java(name) {
        return true;
    }
    let shape = get_collision_shape(block, state);
    if shape.is_empty() {
        return false;
    }
    let bounds = shape.bounds();
    bounds.min_x < 0.0
        || bounds.min_y < 0.0
        || bounds.min_z < 0.0
        || bounds.max_x > 1.0
        || bounds.max_y > 1.0
        || bounds.max_z > 1.0
}
fn is_dynamic_shape_block_like_java(name: &str) -> bool {
    name == "moving_piston"
        || name == "bamboo"
        || name == "scaffolding"
        || name == "powder_snow"
        || name == "pointed_dripstone"
        || name == "sulfur_spike"
        || name.ends_with("shulker_box")
}
fn is_air_like(name: &str) -> bool {
    matches!(
        name,
        "air" | "cave_air" | "void_air" | "light" | "structure_void"
    )
}
fn is_fluid_like(name: &str) -> bool {
    name == "water" || name == "lava"
}
fn liquid_block_collision_shape(context: &CollisionContext) -> VoxelShape {
    if context.always_collide_with_fluid() {
        VoxelShape::cube()
    } else {
        VoxelShape::empty()
    }
}
fn is_collisionless_plant_or_decoration(name: &str) -> bool {
    is_plant_like(name)
        || matches!(
            name,
            "cobweb"
                | "golden_dandelion"
                | "oxeye_daisy"
                | "kelp"
                | "kelp_plant"
                | "seagrass"
                | "tall_seagrass"
                | "sea_pickle"
                | "sugar_cane"
                | "bamboo_sapling"
                | "cactus_flower"
                | "spore_blossom"
                | "hanging_roots"
                | "glow_lichen"
                | "vine"
                | "weeping_vines"
                | "weeping_vines_plant"
                | "twisting_vines"
                | "twisting_vines_plant"
                | "cave_vines"
                | "cave_vines_plant"
                | "torch"
                | "wall_torch"
                | "soul_torch"
                | "soul_wall_torch"
                | "redstone_torch"
                | "redstone_wall_torch"
                | "lever"
                | "redstone_dust"
                | "redstone_wire"
                | "tripwire"
                | "tripwire_hook"
                | "ladder"
                | "lily_pad"
                | "leaf_litter"
                | "rail"
                | "pink_petals"
                | "wildflowers"
                | "firefly_bush"
                | "frogspawn"
                | "resin_clump"
                | "pale_hanging_moss"
                | "mangrove_propagule"
                | "nether_portal"
                | "end_portal"
                | "end_gateway"
                | "bubble_column"
                | "sculk_vein"
                | "fire"
                | "soul_fire"
                | "warped_fungus"
                | "crimson_fungus"
        )
        || name.ends_with("_torch")
        || (name.ends_with("_sign") && !name.ends_with("_wall_hanging_sign"))
        || name.ends_with("_wall_sign")
        || (name.ends_with("_hanging_sign") && !name.ends_with("_wall_hanging_sign"))
        || name.ends_with("_banner")
        || name.ends_with("_wall_banner")
        || name.ends_with("_button")
        || name.ends_with("_pressure_plate")
        || name.ends_with("_rail")
        || name.ends_with("_coral")
        || name.ends_with("_coral_fan")
        || name.ends_with("_coral_wall_fan")
}
fn is_carpet_like(name: &str) -> bool {
    name == "moss_carpet" || name == "pale_moss_carpet" || name.ends_with("_carpet")
}
fn is_iron_bars_or_pane_like(name: &str) -> bool {
    name == "iron_bars"
        || name == "glass_pane"
        || name.ends_with("_bars")
        || name.ends_with("_glass_pane")
}
fn is_vanilla_fence(name: &str) -> bool {
    matches!(
        name,
        "oak_fence"
            | "spruce_fence"
            | "birch_fence"
            | "jungle_fence"
            | "acacia_fence"
            | "cherry_fence"
            | "dark_oak_fence"
            | "pale_oak_fence"
            | "mangrove_fence"
            | "bamboo_fence"
            | "crimson_fence"
            | "warped_fence"
            | "nether_brick_fence"
    )
}
fn is_vanilla_iron_bars_or_pane(name: &str) -> bool {
    matches!(
        name,
        "iron_bars"
            | "copper_bars"
            | "exposed_copper_bars"
            | "weathered_copper_bars"
            | "oxidized_copper_bars"
            | "waxed_copper_bars"
            | "waxed_exposed_copper_bars"
            | "waxed_weathered_copper_bars"
            | "waxed_oxidized_copper_bars"
            | "glass_pane"
            | "white_stained_glass_pane"
            | "orange_stained_glass_pane"
            | "magenta_stained_glass_pane"
            | "light_blue_stained_glass_pane"
            | "yellow_stained_glass_pane"
            | "lime_stained_glass_pane"
            | "pink_stained_glass_pane"
            | "gray_stained_glass_pane"
            | "light_gray_stained_glass_pane"
            | "cyan_stained_glass_pane"
            | "purple_stained_glass_pane"
            | "blue_stained_glass_pane"
            | "brown_stained_glass_pane"
            | "green_stained_glass_pane"
            | "red_stained_glass_pane"
            | "black_stained_glass_pane"
    )
}
fn is_vanilla_transparent_block(name: &str) -> bool {
    matches!(
        name,
        "glass"
            | "tinted_glass"
            | "white_stained_glass"
            | "orange_stained_glass"
            | "magenta_stained_glass"
            | "light_blue_stained_glass"
            | "yellow_stained_glass"
            | "lime_stained_glass"
            | "pink_stained_glass"
            | "gray_stained_glass"
            | "light_gray_stained_glass"
            | "cyan_stained_glass"
            | "purple_stained_glass"
            | "blue_stained_glass"
            | "brown_stained_glass"
            | "green_stained_glass"
            | "red_stained_glass"
            | "black_stained_glass"
            | "copper_grate"
            | "exposed_copper_grate"
            | "weathered_copper_grate"
            | "oxidized_copper_grate"
            | "waxed_copper_grate"
            | "waxed_exposed_copper_grate"
            | "waxed_weathered_copper_grate"
            | "waxed_oxidized_copper_grate"
    )
}
fn is_chain_like(name: &str) -> bool {
    name == "chain" || name.ends_with("_chain")
}
fn is_lantern_like(name: &str) -> bool {
    name == "lantern" || name == "soul_lantern" || name.ends_with("copper_lantern")
}
fn is_no_occlusion_block(name: &str) -> bool {
    is_air_like(name)
        || is_fluid_like(name)
        || is_collisionless_plant_or_decoration(name)
        || name.contains("glass")
        || name.contains("leaves")
        || name.ends_with("_door")
        || name.ends_with("_trapdoor")
        || name.ends_with("_fence")
        || name.ends_with("_wall")
        || name.ends_with("_bars")
        || name.ends_with("_grate")
        || name.ends_with("_sign")
        || name.ends_with("_wall_sign")
        || name.ends_with("_hanging_sign")
        || name.ends_with("_wall_hanging_sign")
        || name.ends_with("_banner")
        || name.ends_with("_wall_banner")
        || name.ends_with("_head")
        || name.ends_with("_wall_head")
        || name.ends_with("_skull")
        || name.ends_with("_wall_skull")
        || name.ends_with("_chain")
        || name.ends_with("_cauldron")
        || name.starts_with("potted_")
        || matches!(
            name,
            "ice"
                | "frosted_ice"
                | "slime_block"
                | "honey_block"
                | "barrier"
                | "bamboo"
                | "chain"
                | "lantern"
                | "soul_lantern"
                | "campfire"
                | "soul_campfire"
                | "snow"
                | "farmland"
                | "dirt_path"
                | "repeater"
                | "comparator"
                | "daylight_detector"
                | "hopper"
                | "cauldron"
                | "composter"
                | "lectern"
                | "grindstone"
                | "stonecutter"
                | "enchanting_table"
                | "anvil"
                | "chipped_anvil"
                | "damaged_anvil"
                | "end_portal_frame"
                | "dragon_egg"
                | "turtle_egg"
                | "sniffer_egg"
                | "piston_head"
                | "brewing_stand"
                | "decorated_pot"
                | "flower_pot"
                | "lightning_rod"
                | "pointed_dripstone"
                | "end_rod"
                | "conduit"
                | "bell"
                | "cake"
                | "candle"
                | "candle_cake"
                | "scaffolding"
                | "spawner"
                | "trial_spawner"
                | "vault"
                | "sculk_sensor"
                | "calibrated_sculk_sensor"
                | "sculk_shrieker"
                | "cocoa"
                | "big_dripleaf"
                | "big_dripleaf_stem"
                | "chorus_plant"
                | "amethyst_cluster"
                | "small_amethyst_bud"
                | "medium_amethyst_bud"
                | "large_amethyst_bud"
        )
}
fn is_plant_like(name: &str) -> bool {
    (name.contains("grass") && name != "grass_block")
        || name.contains("flower")
        || name.contains("sapling")
        || name.contains("fern")
        || name.contains("mushroom")
        || name.contains("roots")
        || name.contains("sprouts")
        || name.contains("bush")
        || name.contains("crop")
        || name.contains("stem")
        || name == "dead_bush"
        || name.contains("tulip")
        || matches!(
            name,
            "dandelion"
                | "golden_dandelion"
                | "poppy"
                | "blue_orchid"
                | "allium"
                | "azure_bluet"
                | "daisy"
                | "cornflower"
                | "lily_of_the_valley"
                | "wither_rose"
                | "wheat"
                | "carrots"
                | "potatoes"
                | "beetroots"
                | "nether_wart"
                | "azalea"
                | "flowering_azalea"
                | "sunflower"
                | "lilac"
                | "rose_bush"
                | "peony"
                | "big_dripleaf"
                | "big_dripleaf_stem"
                | "small_dripleaf"
                | "chorus_flower"
                | "chorus_plant"
                | "pitcher_plant"
                | "pitcher_crop"
                | "torchflower"
                | "torchflower_crop"
                | "closed_eyeblossom"
                | "open_eyeblossom"
        )
}
pub fn get_interaction_shape(block: &Block, state: &BlockState) -> VoxelShape {
    let name = block.location.path.as_str();
    if name == "structure_void" {
        return cube_shape(6.0, 6.0, 6.0);
    }
    if is_air_like(name) {
        return VoxelShape::empty();
    }
    if is_vanilla_fence(name) || name.ends_with("_fence") {
        return fence_outline_shape(state);
    }
    if name.ends_with("_wall") {
        return wall_outline_shape(state);
    }
    if name.ends_with("_fence_gate") {
        return fence_gate_outline_shape(state);
    }
    if name.ends_with("_door") {
        return door_shape(state);
    }
    if name.ends_with("_trapdoor") {
        return trapdoor_shape(state);
    }
    if name == "ladder" {
        return ladder_shape(state);
    }
    if name == "rail" || name.ends_with("_rail") {
        let is_slope = state
            .get_property("shape")
            .map(|shape| shape.starts_with("ascending_"))
            .unwrap_or(false);
        let height = if is_slope { 8.0 / 16.0 } else { 2.0 / 16.0 };
        return VoxelShape::from_aabb(AABB::new(0.0, 0.0, 0.0, 1.0, height, 1.0));
    }
    if name == "lever" {
        return lever_shape(state);
    }
    if name.ends_with("_button") {
        return button_shape(state);
    }
    if name.ends_with("_pressure_plate") {
        return pressure_plate_shape(state);
    }
    if name == "redstone_wire" {
        return redstone_wire_shape(state);
    }
    if matches!(name, "repeater" | "comparator") {
        return column_shape(16.0, 16.0, 0.0, 2.0);
    }
    if name == "daylight_detector" {
        return column_shape(16.0, 16.0, 0.0, 6.0);
    }
    if matches!(name, "sculk_sensor" | "calibrated_sculk_sensor") {
        return column_shape(16.0, 16.0, 0.0, 8.0);
    }
    if name == "heavy_core" {
        return column_shape(8.0, 8.0, 0.0, 8.0);
    }
    if matches!(name, "lightning_rod" | "end_rod") {
        return rod_shape(state);
    }
    if name == "lectern" {
        return lectern_outline_shape(state);
    }
    if name == "bell" {
        return bell_shape(state);
    }
    if is_skull_like(name) {
        return skull_shape(block, state);
    }
    if name.ends_with("_banner") || name.ends_with("_wall_banner") {
        return banner_shape(block, state);
    }
    if name.ends_with("_sign")
        || name.ends_with("_wall_sign")
        || name.ends_with("_hanging_sign")
        || name.ends_with("_wall_hanging_sign")
    {
        return sign_outline_shape(block, state);
    }
    if name == "pointed_dripstone" {
        return pointed_dripstone_shape(state);
    }
    if matches!(
        name,
        "amethyst_cluster" | "large_amethyst_bud" | "medium_amethyst_bud" | "small_amethyst_bud"
    ) {
        return amethyst_cluster_shape(block, state);
    }
    if name == "candle" || name.ends_with("_candle") {
        return candle_shape(state);
    }
    if name == "cake" {
        return cake_collision_shape(state);
    }
    if name == "candle_cake" || name.ends_with("_candle_cake") {
        return candle_cake_collision_shape();
    }
    if name == "decorated_pot" {
        return decorated_pot_shape();
    }
    if is_flower_pot_like(name) {
        return flower_pot_shape();
    }
    if name == "brewing_stand" {
        return brewing_stand_shape();
    }
    if matches!(name, "anvil" | "chipped_anvil" | "damaged_anvil") {
        return anvil_shape(state);
    }
    if name == "stonecutter" {
        return column_shape(16.0, 16.0, 0.0, 9.0);
    }
    if name == "enchanting_table" {
        return column_shape(16.0, 16.0, 0.0, 12.0);
    }
    if name == "end_portal_frame" {
        return end_portal_frame_shape(state);
    }
    if name == "dragon_egg" {
        return column_shape(14.0, 14.0, 0.0, 16.0);
    }
    if name == "conduit" {
        return cube_shape(6.0, 6.0, 6.0);
    }
    if name == "wheat" {
        return crop_shape(state, 7, 2, 2);
    }
    if matches!(name, "carrots" | "potatoes") {
        return crop_shape(state, 7, 2, 2);
    }
    if name == "beetroots" {
        return crop_shape(state, 3, 2, 4);
    }
    if name == "torchflower_crop" {
        return torchflower_crop_shape(state);
    }
    if name == "pitcher_crop" {
        return pitcher_crop_outline_shape(state);
    }
    if name == "nether_wart" {
        return nether_wart_shape(state);
    }
    if matches!(name, "pumpkin_stem" | "melon_stem") {
        return crop_stem_shape(state);
    }
    if matches!(name, "attached_pumpkin_stem" | "attached_melon_stem") {
        return attached_stem_shape(state);
    }
    if name == "cocoa" {
        return cocoa_shape(state);
    }
    if name == "sweet_berry_bush" {
        return sweet_berry_bush_shape(state);
    }
    if name.contains("leaves") {
        return VoxelShape::cube();
    }
    if matches!(
        name,
        "torch" | "soul_torch" | "redstone_torch" | "copper_torch"
    ) {
        return column_shape(4.0, 4.0, 0.0, 10.0);
    }
    if matches!(
        name,
        "wall_torch" | "soul_wall_torch" | "redstone_wall_torch" | "copper_wall_torch"
    ) {
        return wall_torch_shape(state);
    }
    if matches!(name, "brown_mushroom" | "red_mushroom") {
        return column_shape(6.0, 6.0, 0.0, 6.0);
    }
    if matches!(name, "crimson_fungus" | "warped_fungus") {
        return column_shape(8.0, 8.0, 0.0, 9.0);
    }
    if matches!(name, "crimson_roots" | "warped_roots") {
        return column_shape(12.0, 12.0, 0.0, 13.0);
    }
    if matches!(name, "azalea" | "flowering_azalea") {
        return azalea_shape();
    }
    if name == "mangrove_propagule" {
        return mangrove_propagule_shape(state);
    }
    if name == "bamboo_sapling" {
        return column_shape(8.0, 8.0, 0.0, 12.0);
    }
    if name == "bamboo" {
        return bamboo_outline_shape(state);
    }
    if name.ends_with("_sapling") {
        return column_shape(12.0, 12.0, 0.0, 12.0);
    }
    if matches!(name, "short_grass" | "fern") {
        return column_shape(12.0, 12.0, 0.0, 13.0);
    }
    if name == "dead_bush" {
        return column_shape(12.0, 12.0, 0.0, 13.0);
    }
    if name == "nether_sprouts" {
        return column_shape(12.0, 12.0, 0.0, 3.0);
    }
    if name == "sugar_cane" {
        return column_shape(12.0, 12.0, 0.0, 16.0);
    }
    if name == "cactus" {
        return column_shape(14.0, 14.0, 0.0, 16.0);
    }
    if name == "cactus_flower" {
        return column_shape(14.0, 14.0, 0.0, 12.0);
    }
    if name == "leaf_litter" {
        return segmentable_shape(state, 1.0, "segment_amount");
    }
    if matches!(name, "pink_petals" | "wildflowers") {
        return segmentable_shape(state, 3.0, "flower_amount");
    }
    if matches!(
        name,
        "dandelion"
            | "golden_dandelion"
            | "torchflower"
            | "poppy"
            | "blue_orchid"
            | "allium"
            | "azure_bluet"
            | "red_tulip"
            | "orange_tulip"
            | "white_tulip"
            | "pink_tulip"
            | "oxeye_daisy"
            | "cornflower"
            | "lily_of_the_valley"
            | "wither_rose"
            | "closed_eyeblossom"
            | "open_eyeblossom"
    ) {
        return column_shape(6.0, 6.0, 0.0, 10.0);
    }
    if matches!(
        name,
        "sunflower"
            | "lilac"
            | "rose_bush"
            | "peony"
            | "tall_grass"
            | "large_fern"
            | "pitcher_plant"
    ) {
        return VoxelShape::cube();
    }
    if matches!(name, "cave_vines" | "cave_vines_plant") {
        return column_shape(14.0, 14.0, 0.0, 16.0);
    }
    if name == "kelp" {
        return column_shape(16.0, 16.0, 0.0, 9.0);
    }
    if name == "kelp_plant" {
        return VoxelShape::cube();
    }
    if name == "weeping_vines" {
        return column_shape(8.0, 8.0, 9.0, 16.0);
    }
    if name == "weeping_vines_plant" {
        return column_shape(14.0, 14.0, 0.0, 16.0);
    }
    if name == "twisting_vines" {
        return column_shape(8.0, 8.0, 0.0, 15.0);
    }
    if name == "twisting_vines_plant" {
        return column_shape(8.0, 8.0, 0.0, 16.0);
    }
    if name == "seagrass" {
        return column_shape(12.0, 12.0, 0.0, 12.0);
    }
    if name == "tall_seagrass" {
        return column_shape(12.0, 12.0, 0.0, 16.0);
    }
    if name == "big_dripleaf" {
        return big_dripleaf_shape(state, false);
    }
    if name == "big_dripleaf_stem" {
        return big_dripleaf_stem_shape(state);
    }
    if name == "small_dripleaf" {
        return column_shape(12.0, 12.0, 0.0, 13.0);
    }
    if name == "spore_blossom" {
        return column_shape(12.0, 12.0, 13.0, 16.0);
    }
    if name == "hanging_roots" {
        return column_shape(12.0, 12.0, 10.0, 16.0);
    }
    if name == "vine" || matches!(name, "glow_lichen" | "sculk_vein" | "resin_clump") {
        return multiface_shape(state);
    }
    if name == "pale_hanging_moss" {
        return pale_hanging_moss_shape(state);
    }
    if name == "pale_moss_carpet" {
        return pale_moss_carpet_outline_shape(state);
    }
    if name == "chorus_plant" {
        return chorus_plant_shape(state);
    }
    if name == "chorus_flower" {
        return VoxelShape::cube();
    }
    if name == "lily_pad" {
        return column_shape(14.0, 14.0, 0.0, 1.5);
    }
    if name == "snow" {
        return snow_outline_shape(state);
    }
    if name == "sea_pickle" {
        return sea_pickle_shape(state);
    }
    if name == "turtle_egg" {
        return turtle_egg_shape(state);
    }
    if name == "sniffer_egg" {
        return sniffer_egg_shape();
    }
    if name == "frogspawn" {
        return column_shape(16.0, 16.0, 0.0, 1.5);
    }
    let coll = get_collision_shape(block, state);
    if coll.is_empty() {
        return VoxelShape::from_aabb(AABB::new(0.1875, 0.0, 0.1875, 0.8125, 0.625, 0.8125));
    }
    coll
}
pub fn get_outline_shape(block: &Block, state: &BlockState) -> VoxelShape {
    get_outline_shape_with_context_at(block, state, BlockPos::ZERO, None)
}
pub fn get_outline_shape_with_context(
    block: &Block,
    state: &BlockState,
    holding_item: Option<&str>,
) -> VoxelShape {
    get_outline_shape_with_context_at(block, state, BlockPos::ZERO, holding_item)
}
pub fn get_outline_shape_with_collision_context_at(
    block: &Block,
    state: &BlockState,
    pos: BlockPos,
    context: &CollisionContext,
) -> VoxelShape {
    let mut shape = get_outline_shape_with_context_at(block, state, BlockPos::ZERO, None);
    let name = block.location.path.as_str();
    if block.location.namespace == "minecraft" {
        if name == "light" {
            shape = if context.is_holding_item_path("light") {
                VoxelShape::cube()
            } else {
                VoxelShape::empty()
            };
        } else if name == "scaffolding" {
            shape = if context.is_holding_item_path("scaffolding") {
                VoxelShape::cube()
            } else if state
                .get_property("bottom")
                .map(|value| value == "true")
                .unwrap_or(false)
            {
                scaffolding_unstable_shape()
            } else {
                scaffolding_stable_shape()
            };
        }
    }
    const SHAPE_OFFSET_BLOCKS: &[&str] = &[
        "bamboo_sapling",
        "bamboo",
        "mangrove_propagule",
        "pointed_dripstone",
    ];
    let applies_shape_offset = SHAPE_OFFSET_BLOCKS.contains(&name)
        || matches!(
            name,
            "allium"
                | "azure_bluet"
                | "blue_orchid"
                | "cornflower"
                | "dandelion"
                | "lily_of_the_valley"
                | "orange_tulip"
                | "oxeye_daisy"
                | "pink_tulip"
                | "poppy"
                | "red_tulip"
                | "torchflower"
                | "white_tulip"
                | "wither_rose"
                | "open_eyeblossom"
                | "closed_eyeblossom"
        );
    if applies_shape_offset {
        if let Some([offset_x, offset_y, offset_z]) =
            java_26_2_vanilla_block_model_offset(&block.location.to_string(), [pos.x, pos.y, pos.z])
        {
            shape = shape.move_by(
                f64::from(offset_x),
                f64::from(offset_y),
                f64::from(offset_z),
            );
        }
    }
    shape
}
pub fn get_outline_shape_with_context_at(
    block: &Block,
    state: &BlockState,
    _pos: BlockPos,
    holding_item: Option<&str>,
) -> VoxelShape {
    let name = block.location.path.as_str();
    if block.location.namespace == "minecraft" {
        if is_fluid_like(name) || name == "bubble_column" {
            return VoxelShape::empty();
        }
        if name == "powder_snow" {
            return VoxelShape::cube();
        }
        if name == "light" {
            return if holding_item.is_some_and(|item| item == "minecraft:light" || item == "light")
            {
                VoxelShape::cube()
            } else {
                VoxelShape::empty()
            };
        }
        if name == "hopper" {
            return hopper_outline_shape(state);
        }
        if name == "composter" {
            return composter_outline_shape(state);
        }
        if name == "scaffolding" {
            if holding_item
                .is_some_and(|item| item == "minecraft:scaffolding" || item == "scaffolding")
            {
                return VoxelShape::cube();
            }
            return if state
                .get_property("bottom")
                .map(|value| value == "true")
                .unwrap_or(false)
            {
                scaffolding_unstable_shape()
            } else {
                scaffolding_stable_shape()
            };
        }
    }
    get_interaction_shape(block, state)
}
fn hopper_outline_shape(state: &BlockState) -> VoxelShape {
    let px = 1.0 / 16.0;
    let mut boxes = vec![
        AABB::new(0.0, 10.0 * px, 0.0, 1.0, 11.0 * px, 1.0),
        AABB::new(0.0, 11.0 * px, 0.0, 1.0, 1.0, 2.0 * px),
        AABB::new(0.0, 11.0 * px, 14.0 * px, 1.0, 1.0, 1.0),
        AABB::new(0.0, 11.0 * px, 2.0 * px, 2.0 * px, 1.0, 14.0 * px),
        AABB::new(14.0 * px, 11.0 * px, 2.0 * px, 1.0, 1.0, 14.0 * px),
        AABB::new(
            6.0 * px,
            4.0 * px,
            6.0 * px,
            10.0 * px,
            10.0 * px,
            10.0 * px,
        ),
    ];
    let spout = AABB::new(6.0 * px, 4.0 * px, 0.0, 10.0 * px, 8.0 * px, 8.0 * px);
    let pivot = (8.0 * px, 6.0 * px, 8.0 * px);
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("down");
    let rotated_spout = match facing {
        "east" => rotate_aabb_y_about(spout, pivot, 1),
        "south" => rotate_aabb_y_about(spout, pivot, 2),
        "west" => rotate_aabb_y_about(spout, pivot, 3),
        "up" => rotate_aabb_x_about(spout, pivot, 3),
        "down" => rotate_aabb_x_about(spout, pivot, 1),
        _ => spout,
    };
    boxes.push(AABB::new(
        rotated_spout.min_x.max(0.0),
        rotated_spout.min_y.max(0.0),
        rotated_spout.min_z.max(0.0),
        rotated_spout.max_x.min(1.0),
        rotated_spout.max_y.min(1.0),
        rotated_spout.max_z.min(1.0),
    ));
    VoxelShape::from_aabbs(boxes)
}
fn map_aabb(aabb: AABB, map: impl Fn(f64, f64, f64) -> (f64, f64, f64)) -> AABB {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut min_z = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut max_z = f64::NEG_INFINITY;
    for x in [aabb.min_x, aabb.max_x] {
        for y in [aabb.min_y, aabb.max_y] {
            for z in [aabb.min_z, aabb.max_z] {
                let (mapped_x, mapped_y, mapped_z) = map(x, y, z);
                min_x = min_x.min(mapped_x);
                min_y = min_y.min(mapped_y);
                min_z = min_z.min(mapped_z);
                max_x = max_x.max(mapped_x);
                max_y = max_y.max(mapped_y);
                max_z = max_z.max(mapped_z);
            }
        }
    }
    AABB::new(min_x, min_y, min_z, max_x, max_y, max_z)
}
fn rotate_aabb_y_about(aabb: AABB, (cx, _cy, cz): (f64, f64, f64), quarter_turns: usize) -> AABB {
    map_aabb(aabb, |x, y, z| match quarter_turns % 4 {
        1 => (cx + cz - z, y, cz + x - cx),
        2 => (2.0 * cx - x, y, 2.0 * cz - z),
        3 => (cx + z - cz, y, cz + cx - x),
        _ => (x, y, z),
    })
}
fn rotate_aabb_x_about(aabb: AABB, (_cx, cy, cz): (f64, f64, f64), quarter_turns: usize) -> AABB {
    map_aabb(aabb, |x, y, z| match quarter_turns % 4 {
        1 => (x, cy + z - cz, cz + cy - y),
        2 => (x, 2.0 * cy - y, 2.0 * cz - z),
        3 => (x, cy + cz - z, cz + y - cy),
        _ => (x, y, z),
    })
}
fn composter_outline_shape(state: &BlockState) -> VoxelShape {
    let level = state
        .get_property("level")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(0)
        .clamp(0, 8)
        .min(7);
    let inner_min_y = f64::from((1 + level * 2).clamp(2, 16)) / 16.0;
    let inner_min = 2.0 / 16.0;
    let inner_max = 14.0 / 16.0;
    let mut boxes = vec![AABB::new(0.0, 0.0, 0.0, 1.0, inner_min_y, 1.0)];
    boxes.extend([
        AABB::new(0.0, inner_min_y, 0.0, 1.0, 1.0, inner_min),
        AABB::new(0.0, inner_min_y, inner_max, 1.0, 1.0, 1.0),
        AABB::new(0.0, inner_min_y, inner_min, inner_max, 1.0, inner_max),
        AABB::new(inner_max, inner_min_y, inner_min, 1.0, 1.0, inner_max),
    ]);
    VoxelShape::from_aabbs(boxes)
}
pub fn get_interaction_override_shape(block: &Block, state: &BlockState) -> VoxelShape {
    let name = block.location.path.as_str();
    if block.location.namespace != "minecraft" {
        return VoxelShape::empty();
    }
    if name == "hopper" {
        let mut boxes = column_shape(12.0, 12.0, 11.0, 16.0).to_aabbs().to_vec();
        boxes.push(AABB::new(
            6.0 / 16.0,
            8.0 / 16.0,
            0.0,
            10.0 / 16.0,
            10.0 / 16.0,
            4.0 / 16.0,
        ));
        let facing = state
            .get_property("facing")
            .map(String::as_str)
            .unwrap_or("down");
        return if facing == "down" || facing == "up" {
            VoxelShape::from_aabbs(boxes)
        } else {
            VoxelShape::from_aabbs(rotate_boxes_horizontal(
                &boxes,
                horizontal_rotation_steps(facing),
            ))
        };
    }
    if matches!(
        name,
        "cauldron" | "water_cauldron" | "lava_cauldron" | "powder_snow_cauldron"
    ) {
        return column_shape(12.0, 12.0, 4.0, 16.0);
    }
    if name == "composter" || name == "scaffolding" {
        return VoxelShape::cube();
    }
    VoxelShape::empty()
}
fn wall_torch_shape(state: &BlockState) -> VoxelShape {
    let facing = state
        .get_property("facing")
        .map(String::as_str)
        .unwrap_or("north");
    let north = AABB::new(
        5.5 / 16.0,
        3.0 / 16.0,
        11.0 / 16.0,
        10.5 / 16.0,
        13.0 / 16.0,
        1.0,
    );
    let shape = match facing {
        "south" => AABB::new(
            5.5 / 16.0,
            3.0 / 16.0,
            0.0,
            10.5 / 16.0,
            13.0 / 16.0,
            5.0 / 16.0,
        ),
        "west" => AABB::new(
            11.0 / 16.0,
            3.0 / 16.0,
            5.5 / 16.0,
            1.0,
            13.0 / 16.0,
            10.5 / 16.0,
        ),
        "east" => AABB::new(
            0.0,
            3.0 / 16.0,
            5.5 / 16.0,
            5.0 / 16.0,
            13.0 / 16.0,
            10.5 / 16.0,
        ),
        _ => north,
    };
    VoxelShape::from_aabb(shape)
}
fn is_flower_pot_like(name: &str) -> bool {
    name == "flower_pot" || name.starts_with("potted_")
}
fn flower_pot_shape() -> VoxelShape {
    VoxelShape::from_aabb(AABB::new(
        5.0 / 16.0,
        0.0,
        5.0 / 16.0,
        11.0 / 16.0,
        6.0 / 16.0,
        11.0 / 16.0,
    ))
}
fn powder_snow_collision_shape(pos: BlockPos, context: &CollisionContext) -> VoxelShape {
    if context.is_placement() || !context.has_entity() {
        return VoxelShape::empty();
    }
    if context.fall_distance() > 2.5 {
        return VoxelShape::from_aabb(AABB::new(0.0, 0.0, 0.0, 1.0, 0.9, 1.0));
    }
    if context.is_falling_block()
        || (context.can_entity_walk_on_powder_snow()
            && context.is_above(&VoxelShape::cube(), pos, false)
            && !context.is_descending())
    {
        return VoxelShape::cube();
    }
    VoxelShape::empty()
}
fn scaffolding_collision_shape(
    state: &BlockState,
    pos: BlockPos,
    context: &CollisionContext,
) -> VoxelShape {
    if context.is_placement() {
        return VoxelShape::empty();
    }
    if context.is_above(&VoxelShape::cube(), pos, true) && !context.is_descending() {
        return scaffolding_stable_shape();
    }
    let distance = state
        .get_property("distance")
        .and_then(|value| value.parse::<i32>().ok())
        .unwrap_or(7);
    let bottom = state
        .get_property("bottom")
        .map(|value| value == "true")
        .unwrap_or(false);
    if distance != 0 && bottom && context.is_above(&scaffolding_below_block_shape(), pos, true) {
        scaffolding_unstable_bottom_shape()
    } else {
        VoxelShape::empty()
    }
}
fn scaffolding_stable_shape() -> VoxelShape {
    let px = 1.0 / 16.0;
    VoxelShape::from_aabbs(vec![
        AABB::new(0.0, 14.0 * px, 0.0, 1.0, 1.0, 1.0),
        AABB::new(0.0, 0.0, 0.0, 2.0 * px, 1.0, 2.0 * px),
        AABB::new(14.0 * px, 0.0, 0.0, 1.0, 1.0, 2.0 * px),
        AABB::new(14.0 * px, 0.0, 14.0 * px, 1.0, 1.0, 1.0),
        AABB::new(0.0, 0.0, 14.0 * px, 2.0 * px, 1.0, 1.0),
    ])
}
fn scaffolding_unstable_bottom_shape() -> VoxelShape {
    VoxelShape::from_aabb(AABB::new(0.0, 0.0, 0.0, 1.0, 2.0 / 16.0, 1.0))
}
#[allow(dead_code)]
fn scaffolding_unstable_shape() -> VoxelShape {
    let px = 1.0 / 16.0;
    let mut boxes = scaffolding_stable_shape().to_aabbs().to_vec();
    boxes.extend(
        scaffolding_unstable_bottom_shape()
            .to_aabbs()
            .iter()
            .copied(),
    );
    boxes.extend([
        AABB::new(0.0, 0.0, 0.0, 1.0, 2.0 * px, 2.0 * px),
        AABB::new(14.0 * px, 0.0, 0.0, 1.0, 2.0 * px, 1.0),
        AABB::new(0.0, 0.0, 14.0 * px, 1.0, 2.0 * px, 1.0),
        AABB::new(0.0, 0.0, 0.0, 2.0 * px, 2.0 * px, 1.0),
    ]);
    VoxelShape::from_aabbs(boxes)
}
fn scaffolding_below_block_shape() -> VoxelShape {
    VoxelShape::cube().move_by(0.0, -1.0, 0.0)
}
pub fn is_random_ticking(block: &Block, _state: &BlockState) -> bool {
    let name = block.location.path.as_str();
    name.contains("leaves")
        || name.contains("sapling")
        || name.contains("crop")
        || name == "wheat"
        || name == "carrots"
        || name == "potatoes"
        || name == "beetroots"
        || name == "grass_block"
        || name == "mycelium"
        || name == "nether_wart"
        || name.contains("budding")
}
pub fn is_replaceable(block: &Block, _state: &BlockState) -> bool {
    block.location.namespace == "minecraft"
        && is_java_26_2_vanilla_block_in_tag(&block.location.to_string(), "minecraft:replaceable")
}
pub fn destroy_speed(block: &Block, _state: &BlockState) -> f32 {
    let block_id = block.location.to_string();
    if let Some((destroy_speed, _)) = java_26_2_vanilla_block_mining_properties(&block_id) {
        return destroy_speed;
    }
    1.0
}
pub fn friction(block: &Block, _state: &BlockState) -> f32 {
    match block.location.path.as_str() {
        "ice" | "packed_ice" | "frosted_ice" => 0.98,
        "slime_block" => 0.8,
        "blue_ice" => 0.989,
        _ => 0.6,
    }
}
pub fn is_redstone_conductor(block: &Block, state: &BlockState) -> bool {
    let name = block.location.path.as_str();
    if name.contains("air")
        || name.contains("glass")
        || name.contains("leaves")
        || name.contains("water")
        || name.contains("lava")
        || name.contains("slab")
        || name.contains("stairs")
        || name.contains("fence")
        || name.contains("wall")
        || name.contains("door")
        || name.contains("trapdoor")
        || name.contains("bed")
        || name.contains("chest")
        || name.contains("hopper")
        || name.contains("piston")
        || name.contains("shulker")
        || name.contains("redstone")
        || name.contains("torch")
        || name.contains("repeater")
        || name.contains("comparator")
        || name.contains("button")
        || name.contains("lever")
        || name.contains("observer")
        || name.contains("pressure_plate")
        || name.contains("glowstone")
        || name.contains("ice")
        || name == "slime_block"
        || name == "honey_block"
    {
        return false;
    }
    let shape = get_collision_shape(block, state);
    if shape.is_empty() {
        return false;
    }
    let bounds = shape.bounds();
    (bounds.min_x == 0.0 && bounds.max_x == 1.0)
        && (bounds.min_y == 0.0 && bounds.max_y == 1.0)
        && (bounds.min_z == 0.0 && bounds.max_z == 1.0)
}
pub fn is_signal_source(block: &Block, _state: &BlockState) -> bool {
    let name = block.location.path.as_str();
    name == "redstone_wire"
        || name == "redstone_torch"
        || name == "redstone_wall_torch"
        || name == "repeater"
        || name == "comparator"
        || name == "lever"
        || name == "stone_button"
        || name.ends_with("_button")
        || name.contains("pressure_plate")
        || name == "observer"
        || name == "redstone_block"
}
pub fn get_signal(block: &Block, state: &BlockState, direction: el_math::Direction) -> i32 {
    let name = block.location.path.as_str();
    if !is_signal_source(block, state) {
        return 0;
    }
    match name {
        "redstone_block" => 15,
        "redstone_wire" => {
            if direction == el_math::Direction::Up {
                return 0;
            }
            let power = state
                .get_property("power")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);
            if direction == el_math::Direction::Down {
                return power;
            }
            let conn = state
                .get_property(direction.opposite().name())
                .map(|s| s.as_str())
                .unwrap_or("none");
            if conn != "none" { power } else { 0 }
        }
        "redstone_torch" | "redstone_wall_torch" => {
            let lit = state
                .get_property("lit")
                .map(|s| s == "true")
                .unwrap_or(true);
            if lit {
                if name == "redstone_torch" {
                    if direction != el_math::Direction::Up {
                        return 15;
                    }
                } else {
                    let facing = state
                        .get_property("facing")
                        .map(|s| s.as_str())
                        .unwrap_or("north");
                    if let Some(facing_dir) = el_math::Direction::from_name(facing) {
                        if facing_dir != direction {
                            return 15;
                        }
                    }
                }
            }
            0
        }
        "repeater" => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered {
                let facing = state
                    .get_property("facing")
                    .map(|s| s.as_str())
                    .unwrap_or("north");
                if let Some(facing_dir) = el_math::Direction::from_name(facing) {
                    if facing_dir == direction {
                        return 15;
                    }
                }
            }
            0
        }
        "comparator" => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered {
                let facing = state
                    .get_property("facing")
                    .map(|s| s.as_str())
                    .unwrap_or("north");
                if let Some(facing_dir) = el_math::Direction::from_name(facing) {
                    if facing_dir == direction {
                        return 15;
                    }
                }
            }
            0
        }
        "lever" => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered { 15 } else { 0 }
        }
        _ if name.ends_with("_button") || name == "stone_button" => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered { 15 } else { 0 }
        }
        _ if name.contains("pressure_plate") => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered { 15 } else { 0 }
        }
        "observer" => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered {
                let facing = state
                    .get_property("facing")
                    .map(|s| s.as_str())
                    .unwrap_or("north");
                if let Some(facing_dir) = el_math::Direction::from_name(facing) {
                    if facing_dir == direction {
                        return 15;
                    }
                }
            }
            0
        }
        _ => 0,
    }
}
pub fn get_direct_signal(block: &Block, state: &BlockState, direction: el_math::Direction) -> i32 {
    let name = block.location.path.as_str();
    if !is_signal_source(block, state) {
        return 0;
    }
    match name {
        "redstone_torch" | "redstone_wall_torch" => {
            if direction == el_math::Direction::Down {
                get_signal(block, state, direction)
            } else {
                0
            }
        }
        "repeater" | "comparator" | "observer" => get_signal(block, state, direction),
        "lever" | "stone_button" => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered {
                let attach_dir = get_attachment_direction(state);
                if direction == attach_dir {
                    return 15;
                }
            }
            0
        }
        _ if name.ends_with("_button") => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered {
                let attach_dir = get_attachment_direction(state);
                if direction == attach_dir {
                    return 15;
                }
            }
            0
        }
        _ if name.contains("pressure_plate") => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered && direction == el_math::Direction::Down {
                return 15;
            }
            0
        }
        _ => 0,
    }
}
pub fn own_signal(block: &Block, state: &BlockState) -> i32 {
    let name = block.location.path.as_str();
    match name {
        "redstone_block" => 15,
        "redstone_wire" => state
            .get_property("power")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0),
        "redstone_torch" | "redstone_wall_torch" => {
            let lit = state
                .get_property("lit")
                .map(|s| s == "true")
                .unwrap_or(true);
            if lit { 15 } else { 0 }
        }
        "repeater" | "comparator" | "observer" | "lever" => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered { 15 } else { 0 }
        }
        _ if name.ends_with("_button") || name == "stone_button" => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered { 15 } else { 0 }
        }
        _ if name.contains("pressure_plate") => {
            let powered = state
                .get_property("powered")
                .map(|s| s == "true")
                .unwrap_or(false);
            if powered { 15 } else { 0 }
        }
        _ => 0,
    }
}
fn get_attachment_direction(state: &BlockState) -> el_math::Direction {
    let face = state
        .get_property("face")
        .map(|s| s.as_str())
        .unwrap_or("floor");
    match face {
        "floor" => el_math::Direction::Down,
        "ceiling" => el_math::Direction::Up,
        _ => {
            let facing = state
                .get_property("facing")
                .map(|s| s.as_str())
                .unwrap_or("north");
            if let Some(dir) = el_math::Direction::from_name(facing) {
                dir.opposite()
            } else {
                el_math::Direction::Down
            }
        }
    }
}
