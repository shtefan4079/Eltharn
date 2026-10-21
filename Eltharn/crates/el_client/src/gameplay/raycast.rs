#![allow(dead_code)]
use crate::world::client_level::ClientLevel;
use el_core::collision_context::CollisionContext;
use el_math::AABB;
use el_math::Direction;
use el_math::block_pos::BlockPos;
use glam::Vec3;
#[derive(Clone, Debug, PartialEq)]
pub struct HitResult {
    pub pos: BlockPos,
    pub face: Direction,
    pub hit_pos: Vec3,
}
pub fn raycast_block(
    level: &ClientLevel,
    origin: Vec3,
    direction: Vec3,
    max_dist: f32,
) -> Option<HitResult> {
    raycast_block_with_context(level, origin, direction, max_dist, None)
}
pub fn raycast_block_with_context(
    level: &ClientLevel,
    origin: Vec3,
    direction: Vec3,
    max_dist: f32,
    holding_item: Option<&str>,
) -> Option<HitResult> {
    let context = CollisionContext::empty().with_held_item_path(holding_item.map(str::to_owned));
    raycast_block_with_collision_context(level, origin, direction, max_dist, &context)
}
pub fn raycast_block_with_collision_context(
    level: &ClientLevel,
    origin: Vec3,
    direction: Vec3,
    max_dist: f32,
    context: &CollisionContext,
) -> Option<HitResult> {
    if direction.length_squared() < 1e-6 {
        return None;
    }
    let dir = direction.normalize();
    let end = origin + dir * max_dist;
    let from = origin - dir * (max_dist * 1.0e-7);
    let to = end + dir * (max_dist * 1.0e-7);
    let mut block_x = from.x.floor() as i32;
    let mut block_y = from.y.floor() as i32;
    let mut block_z = from.z.floor() as i32;
    let visit = |block_pos: BlockPos| -> Option<HitResult> {
        let state = level.get_block_state(block_pos);
        let outline = state.get_outline_shape_with_collision_context_at(block_pos, context);
        let (distance, outline_face) =
            closest_shape_hit(origin, dir, max_dist, &outline, block_pos)?;
        let face = match closest_shape_hit(
            origin,
            dir,
            max_dist,
            &state.get_interaction_override_shape(),
            block_pos,
        ) {
            Some((override_distance, override_face)) if override_distance < distance => {
                override_face
            }
            _ => outline_face,
        };
        Some(HitResult {
            pos: block_pos,
            face,
            hit_pos: origin + dir * distance,
        })
    };
    if let Some(hit) = visit(BlockPos::new(block_x, block_y, block_z)) {
        return Some(hit);
    }
    let dx = (to.x - from.x) as f64;
    let dy = (to.y - from.y) as f64;
    let dz = (to.z - from.z) as f64;
    let sign_x = dx.signum() as i32;
    let sign_y = dy.signum() as i32;
    let sign_z = dz.signum() as i32;
    let t_delta_x = if sign_x == 0 {
        f64::INFINITY
    } else {
        sign_x as f64 / dx
    };
    let t_delta_y = if sign_y == 0 {
        f64::INFINITY
    } else {
        sign_y as f64 / dy
    };
    let t_delta_z = if sign_z == 0 {
        f64::INFINITY
    } else {
        sign_z as f64 / dz
    };
    let frac = |value: f64| value - value.floor();
    let mut t_x = t_delta_x
        * if sign_x > 0 {
            1.0 - frac(from.x as f64)
        } else {
            frac(from.x as f64)
        };
    let mut t_y = t_delta_y
        * if sign_y > 0 {
            1.0 - frac(from.y as f64)
        } else {
            frac(from.y as f64)
        };
    let mut t_z = t_delta_z
        * if sign_z > 0 {
            1.0 - frac(from.z as f64)
        } else {
            frac(from.z as f64)
        };
    while t_x <= 1.0 || t_y <= 1.0 || t_z <= 1.0 {
        if t_x < t_y {
            if t_x < t_z {
                block_x += sign_x;
                t_x += t_delta_x;
            } else {
                block_z += sign_z;
                t_z += t_delta_z;
            }
        } else if t_y < t_z {
            block_y += sign_y;
            t_y += t_delta_y;
        } else {
            block_z += sign_z;
            t_z += t_delta_z;
        }
        if let Some(hit) = visit(BlockPos::new(block_x, block_y, block_z)) {
            return Some(hit);
        }
    }
    None
}
fn closest_shape_hit(
    origin: Vec3,
    dir: Vec3,
    max_dist: f32,
    shape: &el_math::VoxelShape,
    block_pos: BlockPos,
) -> Option<(f32, Direction)> {
    let test_point = origin + dir * (max_dist * 0.001);
    let local_test_point = el_math::Vec3::new(
        test_point.x as f64 - block_pos.x as f64,
        test_point.y as f64 - block_pos.y as f64,
        test_point.z as f64 - block_pos.z as f64,
    );
    if shape
        .to_aabbs()
        .iter()
        .any(|aabb| aabb.contains(local_test_point))
    {
        return Some((max_dist * 0.001, approximate_ray_face(dir)));
    }
    shape
        .to_aabbs()
        .iter()
        .filter_map(|local_box| {
            let world_box = AABB::new(
                local_box.min_x + block_pos.x as f64,
                local_box.min_y + block_pos.y as f64,
                local_box.min_z + block_pos.z as f64,
                local_box.max_x + block_pos.x as f64,
                local_box.max_y + block_pos.y as f64,
                local_box.max_z + block_pos.z as f64,
            );
            raycast_aabb(origin, dir, max_dist, &world_box)
        })
        .min_by(|(left, _), (right, _)| left.total_cmp(right))
}
fn approximate_ray_face(dir: Vec3) -> Direction {
    let candidates = [
        (Direction::Down, -dir.y),
        (Direction::Up, dir.y),
        (Direction::North, -dir.z),
        (Direction::South, dir.z),
        (Direction::West, -dir.x),
        (Direction::East, dir.x),
    ];
    let mut nearest = Direction::North;
    let mut highest_dot = f32::MIN_POSITIVE;
    for (direction, dot) in candidates {
        if dot > highest_dot {
            highest_dot = dot;
            nearest = direction;
        }
    }
    nearest.opposite()
}
fn raycast_aabb(origin: Vec3, dir: Vec3, max_dist: f32, aabb: &AABB) -> Option<(f32, Direction)> {
    let mut enter = 0.0_f64;
    let mut exit = max_dist as f64;
    let mut enter_face = None;
    let axes = [
        (
            origin.x as f64,
            dir.x as f64,
            aabb.min_x,
            aabb.max_x,
            Direction::West,
            Direction::East,
        ),
        (
            origin.y as f64,
            dir.y as f64,
            aabb.min_y,
            aabb.max_y,
            Direction::Down,
            Direction::Up,
        ),
        (
            origin.z as f64,
            dir.z as f64,
            aabb.min_z,
            aabb.max_z,
            Direction::North,
            Direction::South,
        ),
    ];
    for (origin_axis, dir_axis, min_axis, max_axis, min_face, max_face) in axes {
        if dir_axis.abs() < 1.0e-9 {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
            continue;
        }
        let t_min = (min_axis - origin_axis) / dir_axis;
        let t_max = (max_axis - origin_axis) / dir_axis;
        let (near, far, near_face) = if t_min <= t_max {
            (t_min, t_max, min_face)
        } else {
            (t_max, t_min, max_face)
        };
        if near > enter {
            enter = near;
            enter_face = Some(near_face);
        }
        exit = exit.min(far);
        if enter > exit {
            return None;
        }
    }
    if exit < 0.0 || enter <= 1.0e-7 || enter > max_dist as f64 {
        return None;
    }
    Some((enter as f32, enter_face?))
}
