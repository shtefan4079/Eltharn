use el_core::block_state::BlockState;
use el_math::block_pos::BlockPos;
use el_protocol_backend::BackendRelativeMovementFlags;
use glam::DVec3;

use super::LocalPlayer;
use crate::gameplay::entity::remote::{
    RemotePositionMoveRotation, absolute_remote_position_move_rotation,
};
use crate::world::client_level::ClientLevel;

pub struct PlayerCorrection {
    pub current: RemotePositionMoveRotation,
    pub old: RemotePositionMoveRotation,
}

impl PlayerCorrection {
    pub fn resolve(
        player: &LocalPlayer,
        change: RemotePositionMoveRotation,
        relative: BackendRelativeMovementFlags,
    ) -> Self {
        Self {
            current: absolute_remote_position_move_rotation(
                RemotePositionMoveRotation {
                    position: player.exact_position(),
                    velocity: player.exact_velocity(),
                    yaw_degrees: player.y_rot_degrees(),
                    pitch_degrees: player.x_rot_degrees(),
                },
                change,
                relative,
            ),
            old: absolute_remote_position_move_rotation(
                RemotePositionMoveRotation {
                    position: player.exact_old_position(),
                    velocity: DVec3::ZERO,
                    yaw_degrees: player.old_y_rot_degrees(),
                    pitch_degrees: player.old_x_rot_degrees(),
                },
                change,
                relative,
            ),
        }
    }

    pub fn apply(&self, player: &mut LocalPlayer) {
        player.set_position_with_old_position_d(self.current.position, self.old.position);
        player.set_velocity_d(self.current.velocity);
        player.set_rotation_degrees_like_java(self.current.yaw_degrees, self.current.pitch_degrees);
        player.set_old_rotation_degrees_raw_like_java(self.old.yaw_degrees, self.old.pitch_degrees);
    }
}

pub fn retain_server_block_update(
    level: &mut ClientLevel,
    pos: BlockPos,
    state: &BlockState,
) -> Option<u32> {
    if level
        .block_state_prediction_handler
        .update_known_server_state(pos, state.clone())
    {
        level
            .block_state_prediction_handler
            .prediction_sequence(pos)
    } else {
        None
    }
}

pub fn apply_authoritative_block_state(level: &mut ClientLevel, pos: BlockPos, state: &BlockState) {
    if level.get_block_state(pos) != *state {
        level.set_block_server_verified(pos, state.clone());
    }
}

pub fn reconcile_predicted_block(
    level: &mut ClientLevel,
    player: &mut LocalPlayer,
    pos: BlockPos,
    state: &BlockState,
    retained_player_pos: DVec3,
) -> bool {
    if level.get_block_state(pos) == *state {
        return false;
    }
    apply_authoritative_block_state(level, pos, state);
    let target = block_prediction_snap_target_like_java(
        state,
        pos,
        &player.collision_context(),
        &player.get_aabb(),
        retained_player_pos,
    );
    if let Some(target) = target {
        player.set_position_d(target);
    }
    target.is_some()
}

pub fn block_prediction_snap_target_like_java(
    state: &BlockState,
    pos: BlockPos,
    context: &el_core::collision_context::CollisionContext,
    bounds: &el_math::AABB,
    retained_player_pos: DVec3,
) -> Option<DVec3> {
    state
        .get_collision_shape_with_context(pos, context)
        .to_aabbs()
        .iter()
        .any(|block_box| {
            bounds.intersects(&block_box.move_by(pos.x as f64, pos.y as f64, pos.z as f64))
        })
        .then_some(retained_player_pos)
}

pub fn replacement_state_for_destroyed_block(
    destroyed: &BlockState,
    air_fallback: &BlockState,
) -> BlockState {
    let fluid_state = destroyed.get_fluid_state();
    if !fluid_state.is_empty() {
        use el_core::fluid_state::FluidKind;
        let path = match fluid_state.kind {
            FluidKind::Water => "water",
            FluidKind::Lava => "lava",
            FluidKind::Empty => "air",
        };
        let block_loc = el_core::resource_location::ResourceLocation::new("minecraft", path);
        let default_id = el_core::registry::BuiltInRegistries::blocks()
            .get_by_location(&block_loc)
            .default_state_id;
        BlockState::new(std::sync::Arc::new(block_loc)).with_id(default_id)
    } else {
        air_fallback.clone()
    }
}

pub fn tick_player_travel(
    player: &mut LocalPlayer,
    level: &ClientLevel,
    input: &crate::input::PlayerInput,
    colliders: &[el_math::VoxelShape],
    mayfly: bool,
    flying_speed: Option<f32>,
) {
    if let Some(speed) = flying_speed {
        player.tick_flying(level, input, speed, colliders, mayfly);
    } else {
        player.tick(level, input, colliders, mayfly);
    }
}
