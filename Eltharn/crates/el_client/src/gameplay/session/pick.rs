use crate::client::state::{
    RemoteEntityRaycastHit, attack_range_values, has_attack_range_component_like_java,
    ordinary_entity_pick_for_interaction,
};
use crate::gameplay::entity::remote::{
    RemoteEntities, is_remote_item_entity, remote_entity_pick_size, remote_entity_type_key,
};
use crate::gameplay::raycast::raycast_block_with_collision_context;
use crate::network::multiplayer::multiplayer_game_mode::MultiPlayerGameMode;
use crate::world::client_level::ClientLevel;
use el_core::collision_context::CollisionContext;
use el_core::item::ItemStack;
use el_math::AABB;
use el_math::Direction;
use el_math::block_pos::BlockPos;
pub struct PickInput<'a> {
    pub origin: glam::Vec3,
    pub direction: glam::Vec3,
    pub block_range: f32,
    pub entity_range: f32,
    pub level: Option<&'a ClientLevel>,
    pub entities: &'a RemoteEntities,
    pub held_item: &'a ItemStack,
    pub collision_context: &'a CollisionContext,
    pub local_player_entity_id: Option<i32>,
    pub crosshair_dir_fallback: Direction,
}
pub struct PickOutput {
    pub block_hit: Option<(BlockPos, Direction)>,
    pub entity_hit: Option<RemoteEntityRaycastHit>,
    pub interaction_target: Option<RemoteEntityRaycastHit>,
    pub can_destroy: bool,
    pub crosshair_dir: Direction,
}
pub fn pick_interaction_target(
    input: &PickInput<'_>,
    game_mode: &MultiPlayerGameMode,
) -> PickOutput {
    let mut entity_range = input.entity_range;
    if game_mode.is_creative() && (entity_range - 3.0).abs() < 0.01 {
        entity_range = 5.0;
    }
    let mut block_hit = None;
    let mut block_hit_distance = f32::INFINITY;
    if let Some(level) = input.level {
        if let Some(hit) = raycast_block_with_collision_context(
            level,
            input.origin,
            input.direction,
            input.block_range,
            input.collision_context,
        ) {
            block_hit_distance = hit.hit_pos.distance(input.origin);
            block_hit = Some((hit.pos, hit.face));
        }
    }
    let has_attack_range = has_attack_range_component_like_java(input.held_item);
    let (min_reach, max_reach, hitbox_margin) =
        attack_range_values(input.held_item, game_mode.is_creative(), entity_range);
    let max_reach_with_margin = max_reach + hitbox_margin;
    let pick_distance = input.block_range.max(max_reach_with_margin);
    let target_entity_hit = pick_remote_entity(
        input.entities,
        input.local_player_entity_id,
        input.origin,
        input.direction,
        pick_distance,
        block_hit_distance,
        hitbox_margin,
    )
    .filter(|hit| hit.distance >= min_reach && hit.distance <= max_reach_with_margin);
    let ordinary_entity_hit =
        ordinary_entity_pick_for_interaction(has_attack_range, target_entity_hit, || {
            pick_remote_entity(
                input.entities,
                input.local_player_entity_id,
                input.origin,
                input.direction,
                input.block_range.max(entity_range),
                block_hit_distance,
                0.0,
            )
        });
    let interaction_target = interaction_target_from_hit(
        input.entities,
        target_entity_hit,
        input.origin,
        entity_range,
    )
    .or_else(|| {
        interaction_target_from_hit(
            input.entities,
            ordinary_entity_hit,
            input.origin,
            entity_range,
        )
    });
    if target_entity_hit.is_some() {
        block_hit = None;
    }
    let crosshair_dir = block_hit
        .as_ref()
        .map(|(_, face)| *face)
        .unwrap_or(input.crosshair_dir_fallback);
    let can_destroy = !game_mode.local_player_mode().is_block_placing_restricted();
    PickOutput {
        block_hit,
        entity_hit: target_entity_hit,
        interaction_target,
        can_destroy,
        crosshair_dir,
    }
}
fn pick_remote_entity(
    entities: &RemoteEntities,
    local_player_entity_id: Option<i32>,
    origin: glam::Vec3,
    direction: glam::Vec3,
    max_dist: f32,
    max_block_distance: f32,
    hitbox_margin: f32,
) -> Option<RemoteEntityRaycastHit> {
    if direction.length_squared() < 1.0e-6 {
        return None;
    }
    let dir = direction.normalize();
    let mut best: Option<RemoteEntityRaycastHit> = None;
    for entity in entities.values() {
        if entity.dead
            || local_player_entity_id == Some(entity.id)
            || is_remote_item_entity(&entity.kind)
        {
            continue;
        }
        let pick_radius = f64::from(hitbox_margin.max(0.0));
        let aabb = remote_entity_pick_aabb(entity, pick_radius);
        let Some(distance) = raycast_remote_entity_aabb(origin, dir, max_dist, &aabb) else {
            continue;
        };
        if distance >= max_block_distance {
            continue;
        }
        if best
            .as_ref()
            .map(|best_hit| distance < best_hit.distance)
            .unwrap_or(true)
        {
            best = Some(RemoteEntityRaycastHit {
                entity_id: entity.id,
                distance,
                location: origin + dir * distance,
            });
        }
    }
    best
}
fn interaction_target_from_hit(
    entities: &RemoteEntities,
    hit: Option<RemoteEntityRaycastHit>,
    eye_position: glam::Vec3,
    interaction_range: f32,
) -> Option<RemoteEntityRaycastHit> {
    let hit = hit?;
    entities
        .get(&hit.entity_id)
        .filter(|entity| {
            remote_entity_type_key(&entity.kind) == "interaction"
                && remote_entity_within_interaction_range(entity, eye_position, interaction_range)
        })
        .map(|_| hit)
}
fn remote_entity_within_interaction_range(
    entity: &crate::gameplay::entity::remote::RemoteEntityRenderState,
    eye_position: glam::Vec3,
    interaction_range: f32,
) -> bool {
    let aabb = remote_entity_pick_aabb(entity, 0.0);
    let eye = el_math::Vec3::new(
        f64::from(eye_position.x),
        f64::from(eye_position.y),
        f64::from(eye_position.z),
    );
    let range = f64::from(interaction_range);
    aabb.distance_to_sq_vec(eye) < range * range
}
fn remote_entity_pick_aabb(
    entity: &crate::gameplay::entity::remote::RemoteEntityRenderState,
    pick_radius: f64,
) -> AABB {
    let pos = entity.position;
    let (width, height) = remote_entity_pick_size(entity);
    let half = f64::from(width) * 0.5 + pick_radius;
    AABB::new(
        pos.x as f64 - half,
        pos.y as f64 - pick_radius,
        pos.z as f64 - half,
        pos.x as f64 + half,
        pos.y as f64 + f64::from(height) + pick_radius,
        pos.z as f64 + half,
    )
}
fn raycast_remote_entity_aabb(
    origin: glam::Vec3,
    dir: glam::Vec3,
    max_dist: f32,
    aabb: &AABB,
) -> Option<f32> {
    if origin.x as f64 >= aabb.min_x
        && origin.x as f64 <= aabb.max_x
        && origin.y as f64 >= aabb.min_y
        && origin.y as f64 <= aabb.max_y
        && origin.z as f64 >= aabb.min_z
        && origin.z as f64 <= aabb.max_z
    {
        return Some(0.0);
    }
    let mut enter = 0.0_f64;
    let mut exit = max_dist as f64;
    let axes = [
        (origin.x as f64, dir.x as f64, aabb.min_x, aabb.max_x),
        (origin.y as f64, dir.y as f64, aabb.min_y, aabb.max_y),
        (origin.z as f64, dir.z as f64, aabb.min_z, aabb.max_z),
    ];
    for (origin_axis, dir_axis, min_axis, max_axis) in axes {
        if dir_axis.abs() < 1.0e-9 {
            if origin_axis < min_axis || origin_axis > max_axis {
                return None;
            }
            continue;
        }
        let t_min = (min_axis - origin_axis) / dir_axis;
        let t_max = (max_axis - origin_axis) / dir_axis;
        let near = t_min.min(t_max);
        let far = t_min.max(t_max);
        enter = enter.max(near);
        exit = exit.min(far);
        if enter > exit {
            return None;
        }
    }
    if exit < 0.0 || enter > max_dist as f64 {
        return None;
    }
    Some(enter.max(0.0) as f32)
}
