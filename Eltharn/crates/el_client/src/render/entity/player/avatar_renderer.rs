#![allow(dead_code)]
use crate::gameplay::game_mode::GameType;
use crate::gameplay::player::LocalPlayer;
use crate::render::baked_model::Vertex;
use crate::render::entity::state::AvatarRenderState;
use crate::render::model::humanoid_model::HumanoidModel;
use crate::render::model::player::player_model::PlayerModel;
use crate::render::model::player::player_model::{self, PlayerModelParts};
use glam::DVec3;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AvatarRenderer {
    model: PlayerModel,
}
impl AvatarRenderer {
    pub const fn new(slim: bool) -> Self {
        Self {
            model: PlayerModel::new(slim),
        }
    }
    pub const fn model(&self) -> PlayerModel {
        self.model
    }
    pub fn extract_render_state(
        player: &LocalPlayer,
        game_type: GameType,
        attack_time: f32,
    ) -> AvatarRenderState {
        AvatarRenderState::from_player(player, game_type, attack_time)
    }
    pub fn setup_anim(
        player: &LocalPlayer,
        game_type: GameType,
        attack_time: f32,
    ) -> PlayerModelParts {
        Self::setup_anim_with_walk(
            player,
            game_type,
            attack_time,
            player.walk_animation_pos,
            player.walk_animation_speed,
        )
    }
    pub fn setup_anim_with_walk(
        player: &LocalPlayer,
        game_type: GameType,
        attack_time: f32,
        walk_animation_pos: f32,
        walk_animation_speed: f32,
    ) -> PlayerModelParts {
        Self::setup_anim_with_walk_and_rot(
            player,
            game_type,
            attack_time,
            walk_animation_pos,
            walk_animation_speed,
            player.body_yaw,
            player.head_yaw,
        )
    }
    pub fn setup_anim_with_walk_and_rot(
        player: &LocalPlayer,
        game_type: GameType,
        attack_time: f32,
        walk_animation_pos: f32,
        walk_animation_speed: f32,
        body_yaw: f32,
        head_yaw: f32,
    ) -> PlayerModelParts {
        let mut state = Self::extract_render_state(player, game_type, attack_time);
        state.humanoid.walk_animation_pos = walk_animation_pos;
        state.humanoid.walk_animation_speed = walk_animation_speed;
        state.humanoid.y_rot = wrap_radians(head_yaw - body_yaw).to_degrees();
        Self::setup_anim_state(&state)
    }
    pub fn setup_anim_state(state: &AvatarRenderState) -> PlayerModelParts {
        let mut parts = HumanoidModel::setup_anim_state(&state.humanoid);
        parts.slim = state.skin.model == crate::gameplay::entity::PlayerModelType::Slim;
        parts.show_body = !state.is_spectator;
        parts.show_hat = state.show_hat;
        parts.show_jacket = state.show_jacket;
        parts.show_left_pants = state.show_left_pants;
        parts.show_right_pants = state.show_right_pants;
        parts.show_left_sleeve = state.show_left_sleeve;
        parts.show_right_sleeve = state.show_right_sleeve;
        parts.show_cape = state.show_cape;
        parts.cape_flap = state.cape_flap;
        parts.cape_lean = state.cape_lean;
        parts.cape_lean2 = state.cape_lean2;
        parts
    }
    pub fn generate_player_mesh(parts: &PlayerModelParts) -> (Vec<Vertex>, Vec<u32>) {
        player_model::generate_player_mesh(parts)
    }
    pub fn render_right_hand_mesh(slim: bool, show_sleeve: bool) -> (Vec<Vertex>, Vec<u32>) {
        player_model::generate_first_person_arm(slim, player_model::HandSide::Right, show_sleeve)
    }
    pub fn render_left_hand_mesh(slim: bool, show_sleeve: bool) -> (Vec<Vertex>, Vec<u32>) {
        player_model::generate_first_person_arm(slim, player_model::HandSide::Left, show_sleeve)
    }
}
pub fn extract_cape_state(
    cloak_o: DVec3,
    cloak: DVec3,
    position_o: DVec3,
    position: DVec3,
    body_yaw_o_rad: f32,
    body_yaw_rad: f32,
    fall_flying_scale: f32,
    bob_o: f32,
    bob: f32,
    walk_dist_o: f32,
    walk_dist: f32,
    partial_ticks: f32,
) -> (f32, f32, f32) {
    let lerp = |o: f64, cur: f64| o + (cur - o) * partial_ticks as f64;
    let delta_x = lerp(cloak_o.x, cloak.x) - lerp(position_o.x, position.x);
    let delta_y = lerp(cloak_o.y, cloak.y) - lerp(position_o.y, position.y);
    let delta_z = lerp(cloak_o.z, cloak.z) - lerp(position_o.z, position.z);
    let body_yaw = rot_lerp(body_yaw_o_rad, body_yaw_rad, partial_ticks);
    let forward_x = body_yaw.sin();
    let forward_z = -body_yaw.cos();
    let mut cape_flap = (delta_y as f32) * 10.0;
    cape_flap = cape_flap.clamp(-6.0, 32.0);
    let mut cape_lean = ((delta_x as f32) * forward_x + (delta_z as f32) * forward_z) * 100.0;
    cape_lean *= 1.0 - fall_flying_scale;
    cape_lean = cape_lean.clamp(0.0, 150.0);
    let mut cape_lean2 = ((delta_x as f32) * forward_z - (delta_z as f32) * forward_x) * 100.0;
    cape_lean2 = cape_lean2.clamp(-20.0, 20.0);
    let bob = bob_o + (bob - bob_o) * partial_ticks;
    let walk_distance = walk_dist_o + (walk_dist - walk_dist_o) * partial_ticks;
    cape_flap += (walk_distance * 6.0).sin() * 32.0 * bob;
    (cape_flap, cape_lean, cape_lean2)
}
fn rot_lerp(prev: f32, cur: f32, partial_ticks: f32) -> f32 {
    prev + wrap_radians(cur - prev) * partial_ticks
}
fn wrap_radians(value: f32) -> f32 {
    (value + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}
