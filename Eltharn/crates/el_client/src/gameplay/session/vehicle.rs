use super::ClientSession;
pub(crate) use crate::gameplay::entity::remote::remote_abstract_boat_aabb;
use crate::gameplay::entity::remote::{
    REMOTE_ABSTRACT_BOAT_HEIGHT, RemoteBoatInputLatch, RemoteBoatStatus, RemoteEntityRenderState,
    collect_hard_entity_colliders, is_remote_abstract_boat, is_remote_abstract_chest_boat,
    is_remote_raft, remote_abstract_boat_collision_context,
    remote_abstract_boat_is_locally_controlled, remote_entity_has_local_player_indirect_passenger,
    remote_entity_type_key,
};
use crate::gameplay::physics::collision;
use crate::world::client_level::ClientLevel;
use el_math::AABB;
use el_math::block_pos::BlockPos;
pub(crate) fn remote_abstract_boat_underwater_status_like_java(
    level: &ClientLevel,
    aabb: &AABB,
) -> Option<RemoteBoatStatus> {
    let max_y = aabb.max_y + 0.001;
    let x0 = aabb.min_x.floor() as i32;
    let x1 = aabb.max_x.ceil() as i32;
    let y0 = aabb.max_y.floor() as i32;
    let y1 = max_y.ceil() as i32;
    let z0 = aabb.min_z.floor() as i32;
    let z1 = aabb.max_z.ceil() as i32;
    let mut underwater = false;
    for x in x0..x1 {
        for y in y0..y1 {
            for z in z0..z1 {
                let pos = BlockPos::new(x, y, z);
                let fluid = level.get_block_state(pos).get_fluid_state();
                if fluid.is_water() && max_y < f64::from(y) + f64::from(level.fluid_height(pos)) {
                    if !fluid.is_source() {
                        return Some(RemoteBoatStatus::UnderFlowingWater);
                    }
                    underwater = true;
                }
            }
        }
    }
    underwater.then_some(RemoteBoatStatus::UnderWater)
}
pub(crate) fn remote_abstract_boat_check_in_water_like_java(
    level: &ClientLevel,
    aabb: &AABB,
) -> (bool, f64) {
    let min_x = aabb.min_x.floor() as i32;
    let max_x = aabb.max_x.ceil() as i32;
    let min_y = aabb.min_y.floor() as i32;
    let max_y = (aabb.min_y + 0.001).ceil() as i32;
    let min_z = aabb.min_z.floor() as i32;
    let max_z = aabb.max_z.ceil() as i32;
    let mut in_water = false;
    let mut water_level = -f64::MAX;
    for x in min_x..max_x {
        for y in min_y..max_y {
            for z in min_z..max_z {
                let pos = BlockPos::new(x, y, z);
                let fluid = level.get_block_state(pos).get_fluid_state();
                if fluid.is_water() {
                    let height = f64::from(y) + f64::from(level.fluid_height(pos));
                    water_level = water_level.max(height);
                    in_water |= aabb.min_y < height;
                }
            }
        }
    }
    (in_water, water_level)
}
pub(crate) fn remote_abstract_boat_ground_friction_like_java(
    level: &ClientLevel,
    aabb: &AABB,
) -> f32 {
    let probe = AABB::new(
        aabb.min_x,
        aabb.min_y - 0.001,
        aabb.min_z,
        aabb.max_x,
        aabb.min_y,
        aabb.max_z,
    );
    let x0 = probe.min_x.floor() as i32 - 1;
    let x1 = probe.max_x.ceil() as i32 + 1;
    let y0 = probe.min_y.floor() as i32 - 1;
    let y1 = probe.max_y.ceil() as i32 + 1;
    let z0 = probe.min_z.floor() as i32 - 1;
    let z1 = probe.max_z.ceil() as i32 + 1;
    let mut friction = 0.0_f32;
    let mut count = 0_u32;
    for x in x0..x1 {
        for z in z0..z1 {
            let edges = u32::from(x == x0 || x == x1 - 1) + u32::from(z == z0 || z == z1 - 1);
            if edges == 2 {
                continue;
            }
            for y in y0..y1 {
                if edges > 0 && (y == y0 || y == y1 - 1) {
                    continue;
                }
                let pos = BlockPos::new(x, y, z);
                let state = level.get_block_state(pos);
                if state.block.path == "lily_pad" {
                    continue;
                }
                let intersects_probe = state
                    .get_collision_shape()
                    .move_by(x as f64, y as f64, z as f64)
                    .to_aabbs()
                    .iter()
                    .any(|shape| shape.intersects(&probe));
                if intersects_probe {
                    friction += state.get_friction();
                    count += 1;
                }
            }
        }
    }
    if count == 0 {
        f32::NAN
    } else {
        friction / count as f32
    }
}
pub(crate) fn remote_abstract_boat_water_level_above_like_java(
    level: &ClientLevel,
    aabb: &AABB,
    last_yd: f64,
) -> f64 {
    let min_x = aabb.min_x.floor() as i32;
    let max_x = aabb.max_x.ceil() as i32;
    let min_y = aabb.max_y.floor() as i32;
    let max_y = (aabb.max_y - last_yd).ceil() as i32;
    let min_z = aabb.min_z.floor() as i32;
    let max_z = aabb.max_z.ceil() as i32;
    for y in min_y..max_y {
        let mut block_height = 0.0_f32;
        for x in min_x..max_x {
            for z in min_z..max_z {
                let pos = BlockPos::new(x, y, z);
                if level.get_block_state(pos).get_fluid_state().is_water() {
                    block_height = block_height.max(level.fluid_height(pos));
                }
            }
        }
        if block_height < 1.0 {
            return f64::from(y) + f64::from(block_height);
        }
    }
    f64::from(max_y + 1)
}
pub(crate) fn remote_abstract_boat_status_like_java(
    level: &ClientLevel,
    boat: &mut RemoteEntityRenderState,
) -> RemoteBoatStatus {
    let aabb = remote_abstract_boat_aabb(boat.boat_physics_position());
    if let Some(status) = remote_abstract_boat_underwater_status_like_java(level, &aabb) {
        boat.boat_water_level = aabb.max_y;
        return status;
    }
    let (in_water, water_level) = remote_abstract_boat_check_in_water_like_java(level, &aabb);
    boat.boat_water_level = water_level;
    if in_water {
        return RemoteBoatStatus::InWater;
    }
    let friction = remote_abstract_boat_ground_friction_like_java(level, &aabb);
    if friction > 0.0 {
        boat.boat_land_friction = friction;
        RemoteBoatStatus::OnLand
    } else {
        RemoteBoatStatus::InAir
    }
}
pub(crate) fn float_remote_abstract_boat_like_java(
    level: &ClientLevel,
    boat: &mut RemoteEntityRenderState,
    entity_colliders: &[el_math::VoxelShape],
    has_player_controller: bool,
) {
    let status = boat.boat_status.unwrap_or(RemoteBoatStatus::InWater);
    let mut vertical_speed = -boat.gravity_from_default_like_java(0.04);
    let mut buoyancy = 0.0_f64;
    let inverse_friction;
    if boat.boat_old_status == Some(RemoteBoatStatus::InAir)
        && status != RemoteBoatStatus::InAir
        && status != RemoteBoatStatus::OnLand
    {
        let position = boat.boat_physics_position();
        let aabb = remote_abstract_boat_aabb(position);
        boat.boat_water_level = aabb.max_y;
        let target_y =
            remote_abstract_boat_water_level_above_like_java(level, &aabb, boat.boat_last_yd)
                - REMOTE_ABSTRACT_BOAT_HEIGHT
                + 0.101;
        let target_box = aabb.move_by(0.0, target_y - position.y, 0.0);
        let context = remote_abstract_boat_collision_context(boat, position.y);
        if level.no_collision_with_context_and_entities(
            Some(&aabb),
            &target_box,
            &context,
            entity_colliders,
        ) {
            boat.set_boat_physics_position(glam::DVec3::new(position.x, target_y, position.z));
            let velocity = boat.boat_physics_velocity();
            boat.set_boat_physics_velocity(glam::DVec3::new(velocity.x, 0.0, velocity.z));
            boat.boat_last_yd = 0.0;
        }
        boat.boat_status = Some(RemoteBoatStatus::InWater);
        return;
    }
    match status {
        RemoteBoatStatus::InWater => {
            buoyancy = (boat.boat_water_level - boat.boat_physics_position().y)
                / REMOTE_ABSTRACT_BOAT_HEIGHT;
            inverse_friction = 0.9;
        }
        RemoteBoatStatus::UnderFlowingWater => {
            vertical_speed = -7.0e-4;
            inverse_friction = 0.9;
        }
        RemoteBoatStatus::UnderWater => {
            buoyancy = 0.01;
            inverse_friction = 0.45;
        }
        RemoteBoatStatus::InAir => {
            inverse_friction = 0.9;
        }
        RemoteBoatStatus::OnLand => {
            inverse_friction = boat.boat_land_friction;
            if has_player_controller {
                boat.boat_land_friction /= 2.0;
            }
        }
    }
    let velocity = boat.boat_physics_velocity();
    let inverse_friction_d = f64::from(inverse_friction);
    let mut next_velocity = glam::DVec3::new(
        velocity.x * inverse_friction_d,
        velocity.y + vertical_speed,
        velocity.z * inverse_friction_d,
    );
    boat.boat_delta_rotation_degrees *= inverse_friction;
    if buoyancy > 0.0 {
        next_velocity.y = (next_velocity.y + buoyancy * (0.04_f64 / 0.65_f64)) * 0.75_f64;
    }
    boat.set_boat_physics_velocity(next_velocity);
}
pub(crate) fn control_remote_abstract_boat_like_java(
    boat: &mut RemoteEntityRenderState,
    input: RemoteBoatInputLatch,
) {
    if boat.passengers.is_empty() {
        return;
    }
    let mut acceleration = 0.0_f32;
    if input.left {
        boat.boat_delta_rotation_degrees -= 1.0;
    }
    if input.right {
        boat.boat_delta_rotation_degrees += 1.0;
    }
    if input.right != input.left && !input.forward && !input.backward {
        acceleration += 0.005;
    }
    boat.set_y_rot_degrees_like_java(boat.y_rot_degrees() + boat.boat_delta_rotation_degrees);
    if input.forward {
        acceleration += 0.04;
    }
    if input.backward {
        acceleration -= 0.005;
    }
    let yaw_radians = boat.y_rot_degrees() * (std::f64::consts::PI / 180.0) as f32;
    let velocity = boat.boat_physics_velocity();
    boat.set_boat_physics_velocity(
        velocity
            + glam::DVec3::new(
                f64::from(el_math::mth::sin(-yaw_radians) * acceleration),
                0.0,
                f64::from(el_math::mth::cos(yaw_radians) * acceleration),
            ),
    );
}
pub(crate) fn remote_abstract_boat_check_supporting_block_like_java(
    level: &ClientLevel,
    boat: &mut RemoteEntityRenderState,
    on_ground: bool,
    movement: Option<glam::DVec3>,
) {
    if on_ground {
        let position = boat.boat_physics_position();
        let bounding_box = remote_abstract_boat_aabb(position);
        let test_area = AABB::new(
            bounding_box.min_x,
            bounding_box.min_y - 1.0e-6,
            bounding_box.min_z,
            bounding_box.max_x,
            bounding_box.min_y,
            bounding_box.max_z,
        );
        let source_position = el_math::Vec3::new(position.x, position.y, position.z);
        let context = remote_abstract_boat_collision_context(boat, position.y);
        let mut supporting_block =
            level.find_supporting_block_with_context(source_position, &test_area, &context);
        if supporting_block.is_some() || boat.boat_on_ground_no_blocks {
            boat.boat_main_supporting_block_pos = supporting_block;
        } else if let Some(movement) = movement {
            let on_ground_collision_test_area = test_area.move_by(-movement.x, 0.0, -movement.z);
            supporting_block = level.find_supporting_block_with_context(
                source_position,
                &on_ground_collision_test_area,
                &context,
            );
            boat.boat_main_supporting_block_pos = supporting_block;
        }
        boat.boat_on_ground_no_blocks = supporting_block.is_none();
    } else {
        boat.boat_on_ground_no_blocks = false;
        boat.boat_main_supporting_block_pos = None;
    }
}
pub(crate) fn remote_abstract_boat_get_on_pos_like_java(
    level: &ClientLevel,
    boat: &RemoteEntityRenderState,
    offset: f64,
) -> BlockPos {
    let position = boat.boat_physics_position();
    if let Some(support_pos) = boat.boat_main_supporting_block_pos {
        if offset <= 1.0e-5 {
            return support_pos;
        }
        let below_state = level.get_block_state(support_pos);
        let below_block = below_state.block.path.as_str();
        let is_fence = below_block.ends_with("_fence");
        let is_wall = below_block.ends_with("_wall");
        let is_fence_gate = below_block.ends_with("_fence_gate");
        if (offset > 0.5 || !is_fence) && !is_wall && !is_fence_gate {
            BlockPos::new(
                support_pos.x,
                (position.y - offset).floor() as i32,
                support_pos.z,
            )
        } else {
            support_pos
        }
    } else {
        BlockPos::new(
            position.x.floor() as i32,
            (position.y - offset).floor() as i32,
            position.z.floor() as i32,
        )
    }
}
pub(crate) fn remote_abstract_boat_vanilla_block_speed_factor(block_path: &str) -> f32 {
    match block_path {
        "soul_sand" | "honey_block" => 0.4,
        _ => 1.0,
    }
}
pub(crate) fn remote_abstract_boat_block_speed_factor_like_java(
    level: &ClientLevel,
    boat: &RemoteEntityRenderState,
) -> f64 {
    let position = boat.boat_physics_position();
    let here_pos = BlockPos::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    let state_here = level.get_block_state(here_pos);
    let speed_here = remote_abstract_boat_vanilla_block_speed_factor(&state_here.block.path);
    let factor = if state_here.block.path == "water"
        || state_here.block.path == "bubble_column"
        || speed_here != 1.0
    {
        speed_here
    } else {
        let below = level.get_block_state(remote_abstract_boat_get_on_pos_like_java(
            level, boat, 0.500_001,
        ));
        remote_abstract_boat_vanilla_block_speed_factor(&below.block.path)
    };
    f64::from(factor)
}
pub(crate) fn remote_abstract_boat_vanilla_block_bounce_restitution(block_path: &str) -> f64 {
    if block_path == "slime_block" {
        1.0
    } else if block_path.ends_with("_bed") {
        0.75
    } else {
        0.0
    }
}
pub(crate) fn remote_abstract_boat_block_suppresses_bounce(block_path: &str) -> bool {
    block_path == "honey_block"
}
pub(crate) fn remote_abstract_boat_restitute_movement_after_collisions_like_java(
    level: &ClientLevel,
    boat: &RemoteEntityRenderState,
    attempted_movement: glam::DVec3,
    resolved_movement: glam::DVec3,
    x_collision: bool,
    z_collision: bool,
    vertical_collision: bool,
) -> glam::DVec3 {
    let suppressing_bounce = boat.shared_flag(1);
    let mut restitution = 0.0_f64;
    let mut velocity_after_collision = attempted_movement;
    if x_collision {
        velocity_after_collision.x = -attempted_movement.x * restitution;
    }
    if z_collision {
        velocity_after_collision.z = -attempted_movement.z * restitution;
    }
    if !vertical_collision {
        return velocity_after_collision;
    }
    if attempted_movement.y < 0.0 {
        let effect_pos = remote_abstract_boat_get_on_pos_like_java(level, boat, 0.2);
        let effect_state = level.get_block_state(effect_pos);
        let effect_block = effect_state.block.path.as_str();
        if -attempted_movement.y >= boat.gravity_from_default_like_java(0.04)
            && !suppressing_bounce
            && !remote_abstract_boat_block_suppresses_bounce(&effect_block)
        {
            restitution =
                remote_abstract_boat_vanilla_block_bounce_restitution(&effect_block) * 0.8;
        }
    }
    if restitution > 0.0 {
        let portion_with_movement = resolved_movement.y / attempted_movement.y;
        let gravity_compensation =
            portion_with_movement * boat.gravity_from_default_like_java(0.04);
        velocity_after_collision.y = (gravity_compensation - attempted_movement.y) * restitution;
    } else {
        velocity_after_collision.y = 0.0;
    }
    velocity_after_collision
}
pub(crate) fn move_remote_abstract_boat_with_collisions_like_java(
    level: &ClientLevel,
    boat: &mut RemoteEntityRenderState,
    extra_colliders: &[el_math::VoxelShape],
) {
    debug_assert!(is_remote_abstract_boat(&boat.kind));
    let position = boat.boat_physics_position();
    let attempted_movement = boat.boat_physics_velocity();
    let bounding_box = remote_abstract_boat_aabb(position);
    let context = remote_abstract_boat_collision_context(boat, position.y);
    let movement = collision::collide_movement_like_java(
        level,
        &bounding_box,
        &context,
        boat.on_ground,
        0.0,
        attempted_movement,
        extra_colliders,
    );
    let movement_length_squared = movement.length_squared();
    if movement_length_squared > 1.0e-7
        || attempted_movement.length_squared() - movement_length_squared < 1.0e-7
    {
        boat.set_boat_physics_position(position + movement);
    }
    let x_collision = !el_math::mth::equal_f64(attempted_movement.x, movement.x);
    let z_collision = !el_math::mth::equal_f64(attempted_movement.z, movement.z);
    let vertical_collision = attempted_movement.y != movement.y;
    let vertical_collision_below = vertical_collision && attempted_movement.y < 0.0;
    boat.on_ground = vertical_collision_below;
    remote_abstract_boat_check_supporting_block_like_java(
        level,
        boat,
        vertical_collision_below,
        Some(movement),
    );
    boat.boat_last_yd = attempted_movement.y;
    let mut velocity = remote_abstract_boat_restitute_movement_after_collisions_like_java(
        level,
        boat,
        attempted_movement,
        movement,
        x_collision,
        z_collision,
        vertical_collision,
    );
    let block_speed_factor = remote_abstract_boat_block_speed_factor_like_java(level, boat);
    velocity.x *= block_speed_factor;
    velocity.z *= block_speed_factor;
    boat.set_boat_physics_velocity(velocity);
}
pub(crate) fn tick_controlled_vehicle(
    session: &mut ClientSession,
    vehicle_id: i32,
) -> Option<(bool, bool)> {
    let level = match &session.level {
        Some(l) => l,
        None => return None,
    };
    let boat_input = session
        .remote_boat_input_latch
        .filter(|latch| latch.vehicle_id == vehicle_id)
        .unwrap_or_else(|| RemoteBoatInputLatch::empty(vehicle_id));
    let local_boat_is_authoritative =
        session
            .remote_entities
            .get(&vehicle_id)
            .is_some_and(|vehicle| {
                remote_abstract_boat_is_locally_controlled(vehicle, session.remote_player_entity_id)
            });
    let boat_entity_colliders = collect_hard_entity_colliders(
        &session.remote_entities,
        session.remote_player_entity_id,
        Some(vehicle_id),
    );
    let vehicle = match session.remote_entities.get_mut(&vehicle_id) {
        Some(v) => v,
        None => return None,
    };
    let is_boat = is_remote_abstract_boat(&vehicle.kind);
    let is_mount = vehicle.kind.contains("horse")
        || vehicle.kind.contains("donkey")
        || vehicle.kind.contains("mule")
        || vehicle.kind.contains("llama")
        || vehicle.kind == "camel";
    if is_boat {
        if !local_boat_is_authoritative {
            vehicle.zero_boat_delta_movement_like_java();
            return None;
        }
        vehicle.begin_client_tick();
        vehicle.boat_old_status = vehicle.boat_status;
        vehicle.boat_status = Some(remote_abstract_boat_status_like_java(level, vehicle));
        vehicle.tick_boat_interpolation_like_java(level, &boat_entity_colliders);
        float_remote_abstract_boat_like_java(level, vehicle, &boat_entity_colliders, true);
        control_remote_abstract_boat_like_java(vehicle, boat_input);
        move_remote_abstract_boat_with_collisions_like_java(level, vehicle, &boat_entity_colliders);
        return Some(boat_input.paddle_state());
    } else if is_mount {
        vehicle.velocity = glam::Vec3::ZERO;
    }
    None
}
pub(crate) fn attach_local_player_to_remote_vehicle(session: &mut ClientSession, vehicle_id: i32) {
    let passenger_vehicle_attachment_y = session.player.vehicle_attachment_y(session.player_flying);
    let Some(local_player_id) = session.remote_player_entity_id else {
        return;
    };
    let Some((
        kind,
        position,
        yaw,
        yaw_degrees,
        boat_delta_rotation_degrees,
        passenger_index,
        passenger_count,
    )) = session
        .remote_entities
        .get(&vehicle_id)
        .and_then(|vehicle| {
            vehicle
                .passengers
                .iter()
                .position(|passenger_id| *passenger_id == local_player_id)
                .map(|passenger_index| {
                    (
                        vehicle.kind.clone(),
                        if is_remote_abstract_boat(&vehicle.kind) {
                            vehicle.boat_physics_position()
                        } else {
                            vehicle.position.as_dvec3()
                        },
                        vehicle.yaw,
                        vehicle.y_rot_degrees(),
                        vehicle.boat_delta_rotation_degrees,
                        passenger_index,
                        vehicle.passengers.len(),
                    )
                })
        })
    else {
        return;
    };
    let offset = vehicle_passenger_offset(
        &kind,
        yaw,
        passenger_vehicle_attachment_y,
        passenger_index,
        passenger_count,
    );
    session
        .player
        .set_position_from_vehicle_attachment_d(position + offset);
    if is_remote_abstract_boat(&kind) {
        session
            .player
            .apply_abstract_boat_rider_rotation_degrees(yaw_degrees, boat_delta_rotation_degrees);
    }
    session.player.finish_passenger_ride_tick();
}
pub(crate) fn latch_remote_boat_input_after_ride_tick(
    session: &mut ClientSession,
    vehicle_id: i32,
) {
    let local_controls_boat = session
        .remote_entities
        .get(&vehicle_id)
        .is_some_and(|vehicle| {
            remote_abstract_boat_is_locally_controlled(vehicle, session.remote_player_entity_id)
        });
    if !local_controls_boat {
        if session
            .remote_boat_input_latch
            .is_some_and(|latch| latch.vehicle_id == vehicle_id)
        {
            session.remote_boat_input_latch = None;
        }
        return;
    }
    session.remote_boat_input_latch = Some(RemoteBoatInputLatch {
        vehicle_id,
        left: session.player_input.left,
        right: session.player_input.right,
        forward: session.player_input.forward,
        backward: session.player_input.backward,
    });
}
pub(crate) fn vehicle_passenger_offset(
    kind: &str,
    yaw_radians: f32,
    passenger_vehicle_attachment_y: f32,
    passenger_index: usize,
    passenger_count: usize,
) -> glam::DVec3 {
    let is_raft = is_remote_raft(kind);
    let is_abstract_boat = is_remote_abstract_boat(kind);
    let is_minecart = kind.contains("minecart");
    let is_horse = kind.contains("horse")
        || kind.contains("donkey")
        || kind.contains("mule")
        || kind.contains("skeleton_horse")
        || kind.contains("zombie_horse");
    let is_llama = kind.contains("llama");
    let is_camel = kind.contains("camel");
    if is_abstract_boat {
        let local_z = if passenger_count > 1 {
            if passenger_index == 0 {
                0.2_f32
            } else {
                -0.6_f32
            }
        } else if is_remote_abstract_chest_boat(kind) {
            0.15_f32
        } else {
            0.0_f32
        };
        let ride_height = if is_raft {
            0.5625_f32 * 0.888_888_9_f32
        } else {
            0.5625_f32 / 3.0_f32
        };
        let angle = -yaw_radians;
        let rotated_x = local_z * angle.sin();
        let rotated_z = local_z * angle.cos();
        glam::DVec3::new(
            f64::from(rotated_x),
            f64::from(ride_height - passenger_vehicle_attachment_y),
            f64::from(rotated_z),
        )
    } else if is_minecart {
        glam::DVec3::new(0.0, 0.1875 - passenger_vehicle_attachment_y as f64, 0.0)
    } else if is_horse {
        let height = if kind.contains("donkey") || kind.contains("mule") {
            1.2125
        } else {
            1.44375
        };
        glam::DVec3::new(0.0, height - passenger_vehicle_attachment_y as f64, 0.0)
    } else if is_llama {
        let local_offset =
            glam::DVec3::new(0.0, 1.37 - passenger_vehicle_attachment_y as f64, -0.3);
        let cos = (-yaw_radians).cos() as f64;
        let sin = (-yaw_radians).sin() as f64;
        glam::DVec3::new(
            local_offset.x * cos + local_offset.z * sin,
            local_offset.y,
            -local_offset.x * sin + local_offset.z * cos,
        )
    } else if is_camel {
        glam::DVec3::new(0.0, 1.8 - passenger_vehicle_attachment_y as f64, 0.0)
    } else {
        glam::DVec3::new(0.0, 0.0, 0.0)
    }
}
pub(crate) fn apply_local_player_after_hard_vehicle_snap(
    session: &mut ClientSession,
    vehicle_id: i32,
) {
    if !remote_entity_has_local_player_indirect_passenger(
        vehicle_id,
        session.remote_player_entity_id,
        &session.remote_entities,
    ) {
        return;
    }
    let Some(local_player_id) = session.remote_player_entity_id else {
        return;
    };
    let Some((
        kind,
        position,
        yaw,
        yaw_degrees,
        boat_delta_rotation_degrees,
        passenger_index,
        passenger_count,
    )) = session.remote_entities.get(&vehicle_id).map(|vehicle| {
        (
            vehicle.kind.clone(),
            if is_remote_abstract_boat(&vehicle.kind) {
                vehicle.boat_physics_position()
            } else {
                vehicle.position.as_dvec3()
            },
            vehicle.yaw,
            vehicle.y_rot_degrees(),
            vehicle.boat_delta_rotation_degrees,
            vehicle
                .passengers
                .iter()
                .position(|passenger_id| *passenger_id == local_player_id),
            vehicle.passengers.len(),
        )
    })
    else {
        return;
    };
    let Some(passenger_index) = passenger_index else {
        session.player.set_old_pos_and_rot();
        return;
    };
    if is_remote_abstract_boat(&kind) {
        let passenger_y = session.player.vehicle_attachment_y(session.player_flying);
        let offset =
            vehicle_passenger_offset(&kind, yaw, passenger_y, passenger_index, passenger_count);
        session.player.set_position_d(position + offset);
        session
            .player
            .apply_abstract_boat_rider_rotation_degrees(yaw_degrees, boat_delta_rotation_degrees);
    } else if remote_entity_type_key(&kind).contains("minecart") {
        let passenger_y = session.player.vehicle_attachment_y(session.player_flying);
        session
            .player
            .set_position_d(position + glam::DVec3::new(0.0, 0.1875 - f64::from(passenger_y), 0.0));
    }
    session.player.set_old_pos_and_rot();
}
