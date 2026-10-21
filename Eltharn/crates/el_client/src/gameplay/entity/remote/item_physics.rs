use super::RemoteEntityRenderState;
use crate::gameplay::physics::collision;
use crate::render::mesher::ChunkMesher;
use crate::world::client_level::ClientLevel;
use el_core::block_behaviour::java_26_2_vanilla_block_fluid_support_properties;
use el_core::collision_context::{CollisionContext, CollisionEntityKind};
use el_math::{AABB, block_pos::BlockPos, direction::Direction};
pub(crate) fn remote_item_entity_aabb(position: glam::DVec3) -> AABB {
    AABB::new(
        position.x - 0.125,
        position.y,
        position.z - 0.125,
        position.x + 0.125,
        position.y + 0.25,
        position.z + 0.125,
    )
}
fn remote_tnt_entity_aabb(position: glam::DVec3) -> AABB {
    let width = 0.98_f32;
    let half_width = f64::from(width * 0.5);
    let height = f64::from(0.98_f32);
    AABB::new(
        position.x - half_width,
        position.y,
        position.z - half_width,
        position.x + half_width,
        position.y + height,
        position.z + half_width,
    )
}
pub(crate) fn tick_remote_tnt_motion_like_java(
    level: &ClientLevel,
    entity: &mut RemoteEntityRenderState,
) {
    debug_assert_eq!(super::types::remote_entity_type_key(&entity.kind), "tnt");
    entity.velocity.y -= entity.gravity_from_default_like_java(0.04) as f32;
    let bounding_box = remote_tnt_entity_aabb(entity.position_d);
    let attempted_movement = entity.velocity.as_dvec3();
    let context = CollisionContext::entity(
        CollisionEntityKind::Entity,
        false,
        bounding_box.min_y,
        None,
        None,
        false,
        0.0,
    );
    let movement = collision::collide_movement_like_java(
        level,
        &bounding_box,
        &context,
        entity.on_ground,
        0.0,
        attempted_movement,
        &[],
    );
    entity.set_render_position_d(entity.position_d + movement);
    entity.on_ground = movement.y != attempted_movement.y && attempted_movement.y < 0.0;
    entity.velocity *= 0.98;
    if entity.on_ground {
        entity.velocity *= glam::Vec3::new(0.7, -0.5, 0.7);
    }
}
pub(crate) struct RemoteItemFluidTrackers {
    pub(crate) water_height: f64,
    pub(crate) lava_height: f64,
    pub(crate) water_current: glam::DVec3,
    pub(crate) lava_current: glam::DVec3,
    pub(crate) water_cells: u32,
    pub(crate) lava_cells: u32,
}
pub(crate) fn fluid_flow_like_java(level: &ClientLevel, pos: BlockPos) -> glam::DVec3 {
    const HORIZONTAL: [Direction; 4] = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    let fluid = level.get_block_state(pos).get_fluid_state();
    if fluid.is_empty() {
        return glam::DVec3::ZERO;
    }
    let mut flow_x = 0.0;
    let mut flow_z = 0.0;
    for direction in HORIZONTAL {
        let neighbor_pos = pos.relative(direction);
        let neighbor = level.get_block_state(neighbor_pos).get_fluid_state();
        if !(neighbor.is_empty() || neighbor.kind == fluid.kind) {
            continue;
        }
        let mut neighbor_height = neighbor.own_height();
        let mut distance = 0.0_f32;
        if neighbor_height == 0.0 {
            if !fluid_blocks_motion_like_java(level, neighbor_pos) {
                let below = level
                    .get_block_state(neighbor_pos.relative(Direction::Down))
                    .get_fluid_state();
                if below.is_empty() || below.kind == fluid.kind {
                    neighbor_height = below.own_height();
                    if neighbor_height > 0.0 {
                        distance = fluid.own_height() - (neighbor_height - 0.8888889);
                    }
                }
            }
        } else if neighbor_height > 0.0 {
            distance = fluid.own_height() - neighbor_height;
        }
        if distance != 0.0 {
            flow_x += f64::from(direction.step_x()) * f64::from(distance);
            flow_z += f64::from(direction.step_z()) * f64::from(distance);
        }
    }
    let mut flow = glam::DVec3::new(flow_x, 0.0, flow_z);
    if fluid.falling {
        for direction in HORIZONTAL {
            if fluid_is_solid_face_like_java(level, pos.relative(direction), direction, fluid)
                || fluid_is_solid_face_like_java(
                    level,
                    pos.relative(Direction::Up).relative(direction),
                    direction,
                    fluid,
                )
            {
                flow = java_normalize_or_zero(flow) + glam::DVec3::new(0.0, -6.0, 0.0);
                break;
            }
        }
    }
    java_normalize_or_zero(flow)
}
pub(crate) fn java_normalize_or_zero(vec: glam::DVec3) -> glam::DVec3 {
    let dist = vec.length();
    if dist < 1.0e-5 {
        glam::DVec3::ZERO
    } else {
        vec / dist
    }
}
pub(crate) fn fluid_blocks_motion_like_java(level: &ClientLevel, pos: BlockPos) -> bool {
    let state = level.get_block_state(pos);
    if let Some(properties) = java_26_2_vanilla_block_fluid_support_properties(&state) {
        return properties.blocks_motion;
    }
    !state.get_collision_shape().is_empty()
}
pub(crate) fn fluid_is_solid_face_like_java(
    level: &ClientLevel,
    pos: BlockPos,
    direction: Direction,
    fluid: el_core::fluid_state::FluidState,
) -> bool {
    let state = level.get_block_state(pos);
    if state.get_fluid_state().kind == fluid.kind {
        return false;
    }
    if direction == Direction::Up {
        return true;
    }
    if let Some(properties) = java_26_2_vanilla_block_fluid_support_properties(&state) {
        if properties.ice_block {
            return false;
        }
        return properties.face_sturdy[direction.data_3d() as usize];
    }
    ChunkMesher::occlusion_shape_covers_face(&state.get_occlusion_shape(), direction)
}
pub(crate) fn remote_item_fluid_interaction(
    level: &ClientLevel,
    aabb: &AABB,
) -> RemoteItemFluidTrackers {
    let interaction_box = aabb.deflate_all(0.001);
    let x0 = interaction_box.min_x.floor() as i32;
    let y0 = interaction_box.min_y.floor() as i32;
    let z0 = interaction_box.min_z.floor() as i32;
    let x1 = interaction_box.max_x.ceil() as i32 - 1;
    let y1 = interaction_box.max_y.ceil() as i32 - 1;
    let z1 = interaction_box.max_z.ceil() as i32 - 1;
    if !level.has_fluid_and_loaded(x0 - 1, y0, z0 - 1, x1 + 1, y1, z1 + 1) {
        return RemoteItemFluidTrackers {
            water_height: 0.0,
            lava_height: 0.0,
            water_current: glam::DVec3::ZERO,
            lava_current: glam::DVec3::ZERO,
            water_cells: 0,
            lava_cells: 0,
        };
    }
    let entity_y = aabb.min_y;
    let mut water_height = 0.0;
    let mut lava_height = 0.0;
    let mut water_current = glam::DVec3::ZERO;
    let mut lava_current = glam::DVec3::ZERO;
    let mut water_cells = 0_u32;
    let mut lava_cells = 0_u32;
    for x in x0..=x1 {
        for y in y0..=y1 {
            for z in z0..=z1 {
                let pos = BlockPos::new(x, y, z);
                let fluid = level.get_block_state(pos).get_fluid_state();
                if fluid.is_empty() {
                    continue;
                }
                let fluid_top = y as f64 + f64::from(level.fluid_height(pos));
                if fluid_top < interaction_box.min_y {
                    continue;
                }
                if fluid.is_water() {
                    water_height = f64::max(fluid_top - entity_y, water_height);
                    let mut flow = fluid_flow_like_java(level, pos);
                    if water_height < 0.4 {
                        flow *= water_height;
                    }
                    water_current += flow;
                    water_cells += 1;
                } else if fluid.is_lava() {
                    lava_height = f64::max(fluid_top - entity_y, lava_height);
                    let mut flow = fluid_flow_like_java(level, pos);
                    if lava_height < 0.4 {
                        flow *= lava_height;
                    }
                    lava_current += flow;
                    lava_cells += 1;
                }
            }
        }
    }
    RemoteItemFluidTrackers {
        water_height,
        lava_height,
        water_current,
        lava_current,
        water_cells,
        lava_cells,
    }
}
pub(crate) fn apply_fluid_current_to_item(
    entity: &mut RemoteEntityRenderState,
    current: glam::DVec3,
    cell_count: u32,
    scale: f64,
) {
    if cell_count == 0 || current.length_squared() < 1.0e-5 {
        return;
    }
    let mut impulse = java_normalize_or_zero(current);
    let old = entity.velocity;
    impulse *= scale;
    if f64::from(old.x).abs() < 0.003
        && f64::from(old.z).abs() < 0.003
        && impulse.length() < 0.0045000000000000005
    {
        impulse = java_normalize_or_zero(impulse) * 0.0045000000000000005;
    }
    entity.velocity.x += impulse.x as f32;
    entity.velocity.y += impulse.y as f32;
    entity.velocity.z += impulse.z as f32;
}
pub(crate) fn tick_remote_item_entity_like_java(
    level: &ClientLevel,
    entity: &mut RemoteEntityRenderState,
    extra_colliders: &[el_math::VoxelShape],
) {
    let trackers =
        remote_item_fluid_interaction(level, &remote_item_entity_aabb(entity.position_d));
    apply_fluid_current_to_item(entity, trackers.water_current, trackers.water_cells, 0.014);
    apply_fluid_current_to_item(
        entity,
        trackers.lava_current,
        trackers.lava_cells,
        0.0023333333333333335,
    );
    if trackers.water_height > 0.0 && trackers.water_height > f64::from(0.1_f32) {
        let vy = entity.velocity.y;
        entity.velocity.x *= 0.99;
        entity.velocity.y = vy + if vy < 0.06 { 5.0e-4 } else { 0.0 };
        entity.velocity.z *= 0.99;
    } else if trackers.lava_height > 0.0 && trackers.lava_height > f64::from(0.1_f32) {
        let vy = entity.velocity.y;
        entity.velocity.x *= 0.95;
        entity.velocity.y = vy + if vy < 0.06 { 5.0e-4 } else { 0.0 };
        entity.velocity.z *= 0.95;
    } else {
        entity.velocity.y -= entity.gravity_from_default_like_java(0.04) as f32;
    }
    let on_ground = entity.on_ground;
    let bounding_box = remote_item_entity_aabb(entity.position_d);
    let horizontal_distance_sqr =
        entity.velocity.x * entity.velocity.x + entity.velocity.z * entity.velocity.z;
    if on_ground
        && !(horizontal_distance_sqr > 1.0E-5)
        && (entity.age_in_ticks + entity.id as u32) % 4 != 0
    {
    } else {
        let attempted_movement = entity.velocity.as_dvec3();
        let context = CollisionContext::entity(
            CollisionEntityKind::Entity,
            false,
            bounding_box.min_y,
            None,
            None,
            false,
            0.0,
        );
        let movement = collision::collide_movement_like_java(
            level,
            &bounding_box,
            &context,
            on_ground,
            0.0,
            attempted_movement,
            extra_colliders,
        );
        entity.set_render_position_d(entity.position_d + movement);
        let vertical_collision_below =
            movement.y != attempted_movement.y && attempted_movement.y < 0.0;
        entity.on_ground = vertical_collision_below;
        let mut friction = 0.98_f32;
        if entity.on_ground {
            let below = BlockPos::new(
                entity.position.x.floor() as i32,
                (entity.position.y - 0.999999).floor() as i32,
                entity.position.z.floor() as i32,
            );
            friction *= level.get_block_state(below).get_friction();
        }
        entity.velocity.x *= friction;
        entity.velocity.y *= 0.98;
        entity.velocity.z *= friction;
        if entity.on_ground && entity.velocity.y < 0.0 {
            entity.velocity.y *= -0.5;
        }
    }
    let trackers =
        remote_item_fluid_interaction(level, &remote_item_entity_aabb(entity.position_d));
    apply_fluid_current_to_item(entity, trackers.water_current, trackers.water_cells, 0.014);
    apply_fluid_current_to_item(
        entity,
        trackers.lava_current,
        trackers.lava_cells,
        0.0023333333333333335,
    );
}
