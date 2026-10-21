#![allow(dead_code)]
use crate::world::client_level::ClientLevel;
use el_core::collision_context::CollisionContext;
use el_math::{AABB, Axis, VoxelShape};
use glam::DVec3;
fn collect_colliders_ignoring_world_border_like_java(
    level: &ClientLevel,
    source_aabb: &AABB,
    query_aabb: &AABB,
    context: &CollisionContext,
    selected_entity_colliders: &[VoxelShape],
) -> Vec<VoxelShape> {
    level.get_collisions_with_context_and_preselected_entities(
        Some(source_aabb),
        query_aabb,
        context,
        selected_entity_colliders,
    )
}
fn axis_step_order(movement: DVec3) -> [Axis; 3] {
    if movement.x.abs() < movement.z.abs() {
        [Axis::Y, Axis::Z, Axis::X]
    } else {
        [Axis::Y, Axis::X, Axis::Z]
    }
}
fn axis_value(vec: DVec3, axis: Axis) -> f64 {
    match axis {
        Axis::X => vec.x,
        Axis::Y => vec.y,
        Axis::Z => vec.z,
    }
}
fn with_axis(mut vec: DVec3, axis: Axis, value: f64) -> DVec3 {
    match axis {
        Axis::X => vec.x = value,
        Axis::Y => vec.y = value,
        Axis::Z => vec.z = value,
    }
    vec
}
pub(crate) fn collide_with_shapes(
    movement: DVec3,
    bounding_box: &AABB,
    shapes: &[VoxelShape],
) -> DVec3 {
    if shapes.is_empty() {
        return movement;
    }
    let mut resolved_movement = DVec3::ZERO;
    for axis in axis_step_order(movement) {
        let axis_movement = axis_value(movement, axis);
        if axis_movement != 0.0 {
            let moved_box = bounding_box.move_by(
                resolved_movement.x,
                resolved_movement.y,
                resolved_movement.z,
            );
            let collision = shapes_collide(axis, &moved_box, shapes, axis_movement);
            resolved_movement = with_axis(resolved_movement, axis, collision);
        }
    }
    resolved_movement
}
fn shapes_collide(axis: Axis, moving: &AABB, shapes: &[VoxelShape], mut distance: f64) -> f64 {
    for shape in shapes {
        if distance.abs() < el_math::voxel_shape::EPSILON {
            return 0.0;
        }
        distance = shape.collide(axis, moving, distance);
    }
    distance
}
fn collect_candidate_step_up_heights(
    bounding_box: &AABB,
    colliders: &[VoxelShape],
    max_step_height: f32,
    step_height_to_skip: f32,
) -> Vec<f32> {
    let mut candidates = Vec::new();
    for collider in colliders {
        for coord in collider.coordinates(Axis::Y) {
            let relative_coord = (coord - bounding_box.min_y) as f32;
            if relative_coord < 0.0 || relative_coord == step_height_to_skip {
                continue;
            }
            if relative_coord > max_step_height {
                break;
            }
            candidates.push(relative_coord);
        }
    }
    candidates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    candidates.dedup_by(|a, b| *a == *b);
    candidates
}
pub(crate) fn collide_movement_like_java(
    level: &ClientLevel,
    bounding_box: &AABB,
    context: &CollisionContext,
    on_ground: bool,
    max_up_step: f32,
    movement: DVec3,
    extra_colliders: &[VoxelShape],
) -> DVec3 {
    let expanded_aabb = bounding_box.expand_towards_xyz(movement.x, movement.y, movement.z);
    let entity_colliders = level.get_entity_collisions_from_shapes(&expanded_aabb, extra_colliders);
    let colliders = collect_colliders_ignoring_world_border_like_java(
        level,
        bounding_box,
        &expanded_aabb,
        context,
        &entity_colliders,
    );
    let movement_step = if movement.length_squared() == 0.0 {
        movement
    } else {
        collide_with_shapes(movement, bounding_box, &colliders)
    };
    let x_collision = movement.x != movement_step.x;
    let y_collision = movement.y != movement_step.y;
    let z_collision = movement.z != movement_step.z;
    let on_ground_after_collision = y_collision && movement.y < 0.0;
    if max_up_step > 0.0 && (on_ground_after_collision || on_ground) && (x_collision || z_collision)
    {
        let grounded_aabb = if on_ground_after_collision {
            bounding_box.move_by(0.0, movement_step.y, 0.0)
        } else {
            *bounding_box
        };
        let max_up_step_f64 = max_up_step as f64;
        let mut step_up_aabb =
            grounded_aabb.expand_towards_xyz(movement.x, max_up_step_f64, movement.z);
        if !on_ground_after_collision {
            step_up_aabb = step_up_aabb.expand_towards_xyz(0.0, f64::from(-1.0e-5_f32), 0.0);
        }
        let step_up_colliders = collect_colliders_ignoring_world_border_like_java(
            level,
            &grounded_aabb,
            &step_up_aabb,
            context,
            &entity_colliders,
        );
        let candidate_step_up_heights = collect_candidate_step_up_heights(
            &grounded_aabb,
            &step_up_colliders,
            max_up_step,
            movement_step.y as f32,
        );
        for candidate_step_up_height in candidate_step_up_heights {
            let step_from_ground = collide_with_shapes(
                DVec3::new(movement.x, candidate_step_up_height as f64, movement.z),
                &grounded_aabb,
                &step_up_colliders,
            );
            let step_horizontal_distance_sqr =
                step_from_ground.x * step_from_ground.x + step_from_ground.z * step_from_ground.z;
            let normal_horizontal_distance_sqr =
                movement_step.x * movement_step.x + movement_step.z * movement_step.z;
            if step_horizontal_distance_sqr > normal_horizontal_distance_sqr {
                let distance_to_ground = bounding_box.min_y - grounded_aabb.min_y;
                return step_from_ground - DVec3::new(0.0, distance_to_ground, 0.0);
            }
        }
    }
    movement_step
}
pub(crate) fn collide_with_shapes_and_step_up(
    bounding_box: &AABB,
    on_ground: bool,
    max_up_step: f32,
    movement: DVec3,
    colliders: &[VoxelShape],
) -> DVec3 {
    let movement_step = if movement.length_squared() == 0.0 {
        movement
    } else {
        collide_with_shapes(movement, bounding_box, colliders)
    };
    let x_collision = movement.x != movement_step.x;
    let y_collision = movement.y != movement_step.y;
    let z_collision = movement.z != movement_step.z;
    let on_ground_after_collision = y_collision && movement.y < 0.0;
    if max_up_step > 0.0 && (on_ground_after_collision || on_ground) && (x_collision || z_collision)
    {
        let grounded_aabb = if on_ground_after_collision {
            bounding_box.move_by(0.0, movement_step.y, 0.0)
        } else {
            *bounding_box
        };
        let max_up_step_f64 = max_up_step as f64;
        let mut _step_up_aabb =
            grounded_aabb.expand_towards_xyz(movement.x, max_up_step_f64, movement.z);
        if !on_ground_after_collision {
            _step_up_aabb = _step_up_aabb.expand_towards_xyz(0.0, -1.0e-5_f64, 0.0);
        }
        let candidate_step_up_heights = collect_candidate_step_up_heights(
            &grounded_aabb,
            colliders,
            max_up_step,
            movement_step.y as f32,
        );
        for candidate_step_up_height in candidate_step_up_heights {
            let step_from_ground = collide_with_shapes(
                DVec3::new(movement.x, candidate_step_up_height as f64, movement.z),
                &grounded_aabb,
                colliders,
            );
            let step_horizontal_distance_sqr =
                step_from_ground.x * step_from_ground.x + step_from_ground.z * step_from_ground.z;
            let normal_horizontal_distance_sqr =
                movement_step.x * movement_step.x + movement_step.z * movement_step.z;
            if step_horizontal_distance_sqr > normal_horizontal_distance_sqr {
                let distance_to_ground = bounding_box.min_y - grounded_aabb.min_y;
                return step_from_ground - DVec3::new(0.0, distance_to_ground, 0.0);
            }
        }
    }
    movement_step
}
