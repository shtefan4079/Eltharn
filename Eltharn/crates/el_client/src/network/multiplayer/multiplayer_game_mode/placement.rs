use super::BlockInteractionSound;
use super::canonicalize_block_state;
use crate::gameplay::player::LocalPlayer;
use crate::gameplay::raycast::HitResult;
use crate::world::client_level::{ChunkPos, ClientLevel};
use el_core::BlockState;
use el_core::chest::{ChestFamily, ChestPlacementContext, state_for_placement};
use el_core::item::{DataComponent, ItemStack};
use el_core::registry::BuiltInRegistries;
use el_math::block_pos::BlockPos;
use el_math::{AABB, Axis, Direction, VoxelShape};
use el_nbt::Tag;
pub(crate) fn in_tag(state: &BlockState, tag: &str) -> bool {
    state.block.namespace == "minecraft"
        && el_core::block_behaviour::is_java_26_2_vanilla_block_in_tag(
            &state.block.to_string(),
            tag,
        )
}
fn property<'a>(state: &'a BlockState, key: &str) -> Option<&'a str> {
    state.get_property(key).map(String::as_str)
}
fn facing(state: &BlockState) -> Option<Direction> {
    property(state, "facing").and_then(Direction::from_name)
}
pub(crate) fn door_toggle_sound(
    state: &BlockState,
    pos: BlockPos,
    opening: bool,
) -> Option<BlockInteractionSound> {
    let trapdoor = in_tag(state, "minecraft:trapdoors");
    if !trapdoor && !in_tag(state, "minecraft:doors") {
        return None;
    }
    let suffix = if trapdoor { "_trapdoor" } else { "_door" };
    let family = state.block.path.strip_suffix(suffix)?;
    let sounds = match (family, trapdoor) {
        ("iron", _) => return None,
        ("cherry", false) => (
            "block.cherry_wood_door.open",
            "block.cherry_wood_door.close",
        ),
        ("cherry", true) => (
            "block.cherry_wood_trapdoor.open",
            "block.cherry_wood_trapdoor.close",
        ),
        ("bamboo", false) => (
            "block.bamboo_wood_door.open",
            "block.bamboo_wood_door.close",
        ),
        ("bamboo", true) => (
            "block.bamboo_wood_trapdoor.open",
            "block.bamboo_wood_trapdoor.close",
        ),
        ("crimson" | "warped", false) => (
            "block.nether_wood_door.open",
            "block.nether_wood_door.close",
        ),
        ("crimson" | "warped", true) => (
            "block.nether_wood_trapdoor.open",
            "block.nether_wood_trapdoor.close",
        ),
        (
            "oak" | "spruce" | "birch" | "acacia" | "jungle" | "dark_oak" | "pale_oak" | "mangrove",
            false,
        ) => ("block.wooden_door.open", "block.wooden_door.close"),
        (
            "oak" | "spruce" | "birch" | "acacia" | "jungle" | "dark_oak" | "pale_oak" | "mangrove",
            true,
        ) => ("block.wooden_trapdoor.open", "block.wooden_trapdoor.close"),
        (
            "copper"
            | "exposed_copper"
            | "weathered_copper"
            | "oxidized_copper"
            | "waxed_copper"
            | "waxed_exposed_copper"
            | "waxed_weathered_copper"
            | "waxed_oxidized_copper",
            false,
        ) => ("block.copper_door.open", "block.copper_door.close"),
        (
            "copper"
            | "exposed_copper"
            | "weathered_copper"
            | "oxidized_copper"
            | "waxed_copper"
            | "waxed_exposed_copper"
            | "waxed_weathered_copper"
            | "waxed_oxidized_copper",
            true,
        ) => ("block.copper_trapdoor.open", "block.copper_trapdoor.close"),
        _ => return None,
    };
    Some(BlockInteractionSound {
        pos,
        sound_event: if opening { sounds.0 } else { sounds.1 },
        volume: 1.0,
        pitch_base: 0.9,
        pitch_random_range: 0.1,
    })
}
pub(crate) fn can_replace(
    existing: &BlockState,
    held_block: &BlockState,
    hit: &HitResult,
    target_pos: BlockPos,
    replacing_clicked: bool,
) -> bool {
    let same_item = existing.block == held_block.block;
    if in_tag(existing, "minecraft:slabs") {
        let slab_type = property(existing, "type");
        if slab_type == Some("double") || !same_item {
            return false;
        }
        if !replacing_clicked {
            return true;
        }
        let above = f64::from(hit.hit_pos.y) - f64::from(target_pos.y) > 0.5;
        return if slab_type == Some("bottom") {
            hit.face == Direction::Up || above && hit.face.axis().is_horizontal()
        } else {
            hit.face == Direction::Down || !above && hit.face.axis().is_horizontal()
        };
    }
    if existing.block.namespace == "minecraft" && existing.block.path == "snow" {
        let layers = property(existing, "layers").and_then(|v| v.parse::<u8>().ok());
        return match layers {
            Some(layers) if same_item && layers < 8 => {
                !replacing_clicked || hit.face == Direction::Up
            }
            Some(1) => true,
            _ => false,
        };
    }
    in_tag(existing, "minecraft:replaceable") && !same_item
}
pub(crate) fn placement_target(
    level: &ClientLevel,
    held_block: &BlockState,
    hit: &HitResult,
) -> Option<(BlockPos, bool)> {
    let replace_clicked = can_replace(
        &level.get_block_state(hit.pos),
        held_block,
        hit,
        hit.pos,
        true,
    );
    let target = if replace_clicked {
        hit.pos
    } else {
        hit.pos.relative(hit.face)
    };
    if !within_build_height(level, target)
        || !level.is_within_bounds(target)
        || (!replace_clicked
            && !can_replace(
                &level.get_block_state(target),
                held_block,
                hit,
                target,
                false,
            ))
    {
        return None;
    }
    Some((target, replace_clicked))
}
fn within_build_height(level: &ClientLevel, pos: BlockPos) -> bool {
    level
        .chunks
        .get(&ChunkPos::from_block(pos))
        .is_some_and(|chunk| {
            let min_y = chunk.min_y_section * 16;
            let max_y_exclusive = min_y + chunk.sections.len() as i32 * 16;
            (min_y..max_y_exclusive).contains(&pos.y)
        })
}
pub(crate) fn state_from_item_properties(mut state: BlockState, item: &ItemStack) -> BlockState {
    let Some(DataComponent::Generic {
        value: Tag::Compound(fields),
        ..
    }) = item.components.get("minecraft:block_state")
    else {
        return state;
    };
    for (key, value) in fields {
        let Tag::String(value) = value else {
            continue;
        };
        if !state.properties.contains_key(key) {
            continue;
        }
        let candidate = state.clone().with_property(key, value);
        if let Some(valid) =
            BuiltInRegistries::get_id_by_state(candidate.block.as_ref(), &candidate.properties)
                .and_then(BuiltInRegistries::get_state_by_id)
        {
            state = valid.clone();
        }
    }
    state
}
pub(crate) fn selectable_slot(
    state: &BlockState,
    hit: &HitResult,
    rows: usize,
    columns: usize,
) -> Option<usize> {
    if facing(state)? != hit.face {
        return None;
    }
    let relative =
        hit.hit_pos - glam::Vec3::new(hit.pos.x as f32, hit.pos.y as f32, hit.pos.z as f32);
    let x = match hit.face {
        Direction::North => 1.0 - relative.x,
        Direction::South => relative.x,
        Direction::West => relative.z,
        Direction::East => 1.0 - relative.z,
        _ => return None,
    };
    let section = |coordinate: f32, count: usize| {
        el_math::mth::floor_f32(coordinate * 16.0 / (16.0 / count as f32))
            .clamp(0, count as i32 - 1) as usize
    };
    Some(section(x, columns) + section(1.0 - relative.y, rows) * columns)
}
pub(crate) fn nearest_directions(player: &LocalPlayer) -> [Direction; 6] {
    let pitch = player.x_rot_degrees() * (std::f64::consts::PI / 180.0) as f32;
    let yaw = -player.y_rot_degrees() * (std::f64::consts::PI / 180.0) as f32;
    let pitch_sin = el_math::mth::sin(pitch);
    let pitch_cos = el_math::mth::cos(pitch);
    let yaw_sin = el_math::mth::sin(yaw);
    let yaw_cos = el_math::mth::cos(yaw);
    let x_positive = yaw_sin > 0.0;
    let y_positive = pitch_sin < 0.0;
    let z_positive = yaw_cos > 0.0;
    let x_yaw = if x_positive { yaw_sin } else { -yaw_sin };
    let y_mag = if y_positive { -pitch_sin } else { pitch_sin };
    let z_yaw = if z_positive { yaw_cos } else { -yaw_cos };
    let x_mag = x_yaw * pitch_cos;
    let z_mag = z_yaw * pitch_cos;
    let x = if x_positive {
        Direction::East
    } else {
        Direction::West
    };
    let y = if y_positive {
        Direction::Up
    } else {
        Direction::Down
    };
    let z = if z_positive {
        Direction::South
    } else {
        Direction::North
    };
    let first = if x_yaw > z_yaw {
        if y_mag > x_mag {
            [y, x, z]
        } else if z_mag > y_mag {
            [x, z, y]
        } else {
            [x, y, z]
        }
    } else if y_mag > z_mag {
        [y, z, x]
    } else if x_mag > y_mag {
        [z, x, y]
    } else {
        [z, y, x]
    };
    [
        first[0],
        first[1],
        first[2],
        first[2].opposite(),
        first[1].opposite(),
        first[0].opposite(),
    ]
}
fn is_pillar(state: &BlockState) -> bool {
    in_tag(state, "minecraft:logs")
        || in_tag(state, "minecraft:bamboo_blocks")
        || state.block.namespace == "minecraft"
            && matches!(
                state.block.path.as_str(),
                "muddy_mangrove_roots"
                    | "basalt"
                    | "polished_basalt"
                    | "quartz_pillar"
                    | "purpur_pillar"
                    | "hay_block"
                    | "bone_block"
                    | "deepslate"
                    | "infested_deepslate"
                    | "ochre_froglight"
                    | "verdant_froglight"
                    | "pearlescent_froglight"
            )
}
pub(crate) fn state_for_context(
    mut state: BlockState,
    level: &ClientLevel,
    pos: BlockPos,
    hit: &HitResult,
    replacing_clicked: bool,
    player: &LocalPlayer,
) -> Option<BlockState> {
    let horizontal = Direction::from_y_rot(f64::from(player.y_rot_degrees()));
    let old_state = level.get_block_state(pos);
    let water = old_state.get_fluid_state().is_water();
    let water_string = if water { "true" } else { "false" };
    let above_half = f64::from(hit.hit_pos.y) - f64::from(pos.y) > 0.5;
    if in_tag(&state, "minecraft:stairs") {
        let bottom = hit.face != Direction::Down && (hit.face == Direction::Up || !above_half);
        state = state
            .with_property("facing", horizontal.name())
            .with_property("half", if bottom { "bottom" } else { "top" })
            .with_property("waterlogged", water_string);
        let shape = stairs_shape(level, pos, &state);
        state = state.with_property("shape", shape);
    } else if in_tag(&state, "minecraft:slabs") {
        if old_state.block == state.block {
            state = old_state
                .with_property("type", "double")
                .with_property("waterlogged", "false");
        } else {
            let bottom = hit.face != Direction::Down && (hit.face == Direction::Up || !above_half);
            state = state
                .with_property("type", if bottom { "bottom" } else { "top" })
                .with_property("waterlogged", water_string);
        }
    } else if in_tag(&state, "minecraft:doors") {
        let upper = pos.relative(Direction::Up);
        if !within_build_height(level, upper)
            || !can_replace(
                &level.get_block_state(upper),
                &state,
                hit,
                pos,
                replacing_clicked,
            )
            || !face_sturdy_full(
                &level.get_block_state(pos.relative(Direction::Down)),
                Direction::Up,
            )
        {
            return None;
        }
        let powered = has_neighbor_signal(level, pos) || has_neighbor_signal(level, upper);
        let hinge = door_hinge(level, pos, horizontal, hit);
        let power_string = if powered { "true" } else { "false" };
        state = state
            .with_property("facing", horizontal.name())
            .with_property("hinge", hinge)
            .with_property("powered", power_string)
            .with_property("open", power_string)
            .with_property("half", "lower");
    } else if in_tag(&state, "minecraft:trapdoors") {
        let (direction, half) = if !replacing_clicked && hit.face.axis().is_horizontal() {
            (hit.face, if above_half { "top" } else { "bottom" })
        } else {
            (
                horizontal.opposite(),
                if hit.face == Direction::Up {
                    "bottom"
                } else {
                    "top"
                },
            )
        };
        let power_string = if has_neighbor_signal(level, pos) {
            "true"
        } else {
            "false"
        };
        state = state
            .with_property("facing", direction.name())
            .with_property("half", half)
            .with_property("open", power_string)
            .with_property("powered", power_string)
            .with_property("waterlogged", water_string);
    } else if is_pillar(&state) || in_tag(&state, "minecraft:chains") {
        if in_tag(&state, "minecraft:chains") {
            state = state.with_property("waterlogged", water_string);
        }
        state = state.with_property("axis", hit.face.axis().name());
    } else if in_tag(&state, "minecraft:lanterns") {
        let mut directions = nearest_directions(player);
        if !replacing_clicked {
            let preferred = hit.face.opposite();
            let index = directions
                .iter()
                .position(|direction| *direction == preferred)?;
            directions[..=index].rotate_right(1);
        }
        let mut support_direction = None;
        for direction in directions
            .into_iter()
            .filter(|direction| direction.axis() == Axis::Y)
        {
            match can_support_center(level, pos.relative(direction), direction.opposite()) {
                Some(true) => {
                    support_direction = Some(direction);
                    break;
                }
                Some(false) => {}
                None => return None,
            }
        }
        let hanging = support_direction? == Direction::Up;
        state = state
            .with_property("hanging", if hanging { "true" } else { "false" })
            .with_property("waterlogged", water_string);
    } else if state.block.namespace == "minecraft" {
        match state.block.path.as_str() {
            "piston" | "sticky_piston" => {
                state = state
                    .with_property("facing", nearest_directions(player)[0].opposite().name())
                    .with_property("extended", "false");
            }
            "observer" => {
                state = state.with_property("facing", nearest_directions(player)[0].name());
            }
            "decorated_pot" => {
                state = state
                    .with_property("facing", horizontal.name())
                    .with_property("waterlogged", water_string)
                    .with_property("cracked", "false");
            }
            "snow" if old_state.block == state.block => {
                let layers = property(&old_state, "layers")?.parse::<u8>().ok()?;
                state = old_state.with_property("layers", &(layers + 1).min(8).to_string());
            }
            _ => {}
        }
    }
    if ChestFamily::from_block(&state.block).is_some() {
        return state_for_placement(
            &|pos| Some(level.get_block_state(pos)),
            &state.block,
            ChestPlacementContext {
                clicked_pos: pos,
                clicked_face: hit.face,
                horizontal_direction: horizontal,
                secondary_use_active: player.is_sneaking,
                replaced_fluid_is_water: water,
            },
        );
    }
    Some(canonicalize_block_state(state))
}
fn stairs_shape(level: &ClientLevel, pos: BlockPos, state: &BlockState) -> &'static str {
    let Some(direction) = facing(state) else {
        return "straight";
    };
    let matches_half = |other: &BlockState| {
        in_tag(other, "minecraft:stairs") && property(other, "half") == property(state, "half")
    };
    let can_take = |side| {
        let side_state = level.get_block_state(pos.relative(side));
        !matches_half(&side_state) || facing(&side_state) != Some(direction)
    };
    let behind = level.get_block_state(pos.relative(direction));
    if matches_half(&behind)
        && let Some(other_facing) = facing(&behind)
        && other_facing.axis() != direction.axis()
        && can_take(other_facing.opposite())
    {
        return if other_facing == direction.counter_clockwise() {
            "outer_left"
        } else {
            "outer_right"
        };
    }
    let front = level.get_block_state(pos.relative(direction.opposite()));
    if matches_half(&front)
        && let Some(other_facing) = facing(&front)
        && other_facing.axis() != direction.axis()
        && can_take(other_facing)
    {
        return if other_facing == direction.counter_clockwise() {
            "inner_left"
        } else {
            "inner_right"
        };
    }
    "straight"
}
fn face_sturdy_full(state: &BlockState, direction: Direction) -> bool {
    el_core::block_behaviour::java_26_2_vanilla_block_fluid_support_properties(state)
        .is_some_and(|properties| properties.face_sturdy[direction.data_3d() as usize])
}
fn collision_shape_full(state: &BlockState) -> bool {
    let shape = state.get_collision_shape();
    let boxes = shape.to_aabbs();
    if boxes.iter().any(|b| {
        b.min_x < 0.0
            || b.min_y < 0.0
            || b.min_z < 0.0
            || b.max_x > 1.0
            || b.max_y > 1.0
            || b.max_z > 1.0
    }) {
        return false;
    }
    let mut xs = vec![0.0, 1.0];
    let mut ys = xs.clone();
    let mut zs = xs.clone();
    for b in boxes {
        xs.extend([b.min_x, b.max_x]);
        ys.extend([b.min_y, b.max_y]);
        zs.extend([b.min_z, b.max_z]);
    }
    for values in [&mut xs, &mut ys, &mut zs] {
        values.sort_by(f64::total_cmp);
        values.dedup();
    }
    xs.windows(2).all(|x| {
        ys.windows(2).all(|y| {
            zs.windows(2).all(|z| {
                let p = [
                    (x[0] + x[1]) * 0.5,
                    (y[0] + y[1]) * 0.5,
                    (z[0] + z[1]) * 0.5,
                ];
                boxes.iter().any(|b| {
                    b.min_x <= p[0]
                        && b.max_x >= p[0]
                        && b.min_y <= p[1]
                        && b.max_y >= p[1]
                        && b.min_z <= p[2]
                        && b.max_z >= p[2]
                })
            })
        })
    })
}
fn door_hinge(
    level: &ClientLevel,
    pos: BlockPos,
    direction: Direction,
    hit: &HitResult,
) -> &'static str {
    let left = level.get_block_state(pos.relative(direction.counter_clockwise()));
    let left_upper = level.get_block_state(
        pos.relative(Direction::Up)
            .relative(direction.counter_clockwise()),
    );
    let right = level.get_block_state(pos.relative(direction.clockwise()));
    let right_upper =
        level.get_block_state(pos.relative(Direction::Up).relative(direction.clockwise()));
    let balance = -i32::from(collision_shape_full(&left))
        - i32::from(collision_shape_full(&left_upper))
        + i32::from(collision_shape_full(&right))
        + i32::from(collision_shape_full(&right_upper));
    let door_left = in_tag(&left, "minecraft:doors") && property(&left, "half") == Some("lower");
    let door_right = in_tag(&right, "minecraft:doors") && property(&right, "half") == Some("lower");
    if (!door_left || door_right) && balance <= 0 {
        if (!door_right || door_left) && balance >= 0 {
            let x = f64::from(hit.hit_pos.x) - f64::from(pos.x);
            let z = f64::from(hit.hit_pos.z) - f64::from(pos.z);
            if (direction.step_x() >= 0 || z >= 0.5)
                && (direction.step_x() <= 0 || z <= 0.5)
                && (direction.step_z() >= 0 || x <= 0.5)
                && (direction.step_z() <= 0 || x >= 0.5)
            {
                "left"
            } else {
                "right"
            }
        } else {
            "left"
        }
    } else {
        "right"
    }
}
fn has_neighbor_signal(level: &ClientLevel, pos: BlockPos) -> bool {
    Direction::VALUES.into_iter().any(|direction| {
        let neighbor = pos.relative(direction);
        let state = level.get_block_state(neighbor);
        state.get_signal(direction) > 0
            || state.is_redstone_conductor()
                && Direction::VALUES.into_iter().any(|side| {
                    level
                        .get_block_state(neighbor.relative(side))
                        .get_direct_signal(side)
                        > 0
                })
    })
}
fn vertical_support_shape(state: &BlockState) -> Option<VoxelShape> {
    if BuiltInRegistries::get_id_by_state(state.block.as_ref(), &state.properties).is_none() {
        return None;
    }
    if in_tag(state, "minecraft:leaves") {
        return Some(VoxelShape::empty());
    }
    if state.block.path.ends_with("shulker_box") {
        return None;
    }
    if in_tag(state, "minecraft:fence_gates") {
        let shape = if property(state, "open") == Some("true") {
            VoxelShape::empty()
        } else if facing(state)?.axis() == Axis::Z {
            VoxelShape::from_aabb(AABB::new(
                0.0,
                5.0 / 16.0,
                6.0 / 16.0,
                1.0,
                1.5,
                10.0 / 16.0,
            ))
        } else {
            VoxelShape::from_aabb(AABB::new(
                6.0 / 16.0,
                5.0 / 16.0,
                0.0,
                10.0 / 16.0,
                1.5,
                1.0,
            ))
        };
        return Some(shape);
    }
    match state.block.path.as_str() {
        "mud" | "soul_sand" => Some(VoxelShape::cube()),
        "snow" => Some(state.get_outline_shape()),
        "chorus_flower" => Some(VoxelShape::from_aabb(AABB::new(
            1.0 / 16.0,
            0.0,
            1.0 / 16.0,
            15.0 / 16.0,
            15.0 / 16.0,
            15.0 / 16.0,
        ))),
        _ if in_tag(state, "minecraft:ceiling_hanging_signs")
            || in_tag(state, "minecraft:wall_hanging_signs") =>
        {
            Some(state.get_outline_shape())
        }
        _ => Some(state.get_collision_shape()),
    }
}
fn can_support_center(level: &ClientLevel, pos: BlockPos, face: Direction) -> Option<bool> {
    let state = level.get_block_state(pos);
    if face == Direction::Down && in_tag(&state, "minecraft:unstable_bottom_center") {
        return Some(false);
    }
    if state.block.path.ends_with("shulker_box") {
        return None;
    }
    if face_sturdy_full(&state, face) {
        return Some(true);
    }
    let shape = vertical_support_shape(&state)?;
    let slice_y = if face == Direction::Up {
        0.9999999
    } else {
        1.0e-7
    };
    let boxes: Vec<_> = shape
        .to_aabbs()
        .iter()
        .filter(|b| b.min_y <= slice_y && b.max_y > slice_y)
        .collect();
    let min = 7.0 / 16.0;
    let max = 9.0 / 16.0;
    let mut xs = vec![min, max];
    let mut zs = xs.clone();
    for b in &boxes {
        xs.extend([b.min_x.clamp(min, max), b.max_x.clamp(min, max)]);
        zs.extend([b.min_z.clamp(min, max), b.max_z.clamp(min, max)]);
    }
    for values in [&mut xs, &mut zs] {
        values.sort_by(f64::total_cmp);
        values.dedup();
    }
    Some(xs.windows(2).all(|x| {
        zs.windows(2).all(|z| {
            let px = (x[0] + x[1]) * 0.5;
            let pz = (z[0] + z[1]) * 0.5;
            boxes
                .iter()
                .any(|b| b.min_x <= px && b.max_x >= px && b.min_z <= pz && b.max_z >= pz)
        })
    }))
}
pub(crate) fn neighbor_state(
    level: &ClientLevel,
    pos: BlockPos,
    state: &BlockState,
    direction_to_changed: Direction,
    changed_state: &BlockState,
) -> Option<BlockState> {
    if in_tag(state, "minecraft:stairs") && direction_to_changed.axis().is_horizontal() {
        return Some(canonicalize_block_state(
            state
                .clone()
                .with_property("shape", stairs_shape(level, pos, state)),
        ));
    }
    if in_tag(state, "minecraft:doors") {
        let lower = property(state, "half") == Some("lower");
        if direction_to_changed.axis() == Axis::Y
            && lower == (direction_to_changed == Direction::Up)
        {
            return Some(
                if in_tag(changed_state, "minecraft:doors")
                    && property(changed_state, "half") != property(state, "half")
                {
                    canonicalize_block_state(
                        changed_state
                            .clone()
                            .with_property("half", if lower { "lower" } else { "upper" }),
                    )
                } else {
                    canonicalize_block_state(level.air.clone())
                },
            );
        }
        if lower
            && direction_to_changed == Direction::Down
            && !face_sturdy_full(changed_state, Direction::Up)
        {
            return Some(canonicalize_block_state(level.air.clone()));
        }
    }
    if in_tag(state, "minecraft:lanterns") {
        let support = if property(state, "hanging") == Some("true") {
            Direction::Up
        } else {
            Direction::Down
        };
        if direction_to_changed == support
            && can_support_center(level, pos.relative(support), support.opposite()) == Some(false)
        {
            return Some(canonicalize_block_state(level.air.clone()));
        }
    }
    None
}
