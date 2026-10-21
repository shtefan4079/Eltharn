use crate::gameplay::entity::{ClientAvatarEntity, ParrotVariant, PlayerSkin};
use crate::gameplay::game_mode::GameType;
use crate::gameplay::player::LocalPlayer;
use crate::render::entity::state::HumanoidRenderState;
use el_core::item::ItemStack;
use std::ops::{Deref, DerefMut};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AvatarRenderState {
    pub humanoid: HumanoidRenderState,
    pub is_spectator: bool,
    pub skin: PlayerSkin,
    pub cape_flap: f32,
    pub cape_lean: f32,
    pub cape_lean2: f32,
    pub arrow_count: u32,
    pub stinger_count: u32,
    pub show_hat: bool,
    pub show_jacket: bool,
    pub show_left_pants: bool,
    pub show_right_pants: bool,
    pub show_left_sleeve: bool,
    pub show_right_sleeve: bool,
    pub show_cape: bool,
    pub fall_flying_time_in_ticks: f32,
    pub should_apply_flying_y_rot: bool,
    pub flying_y_rot: f32,
    pub parrot_on_left_shoulder: Option<ParrotVariant>,
    pub parrot_on_right_shoulder: Option<ParrotVariant>,
    pub id: i32,
    pub show_extra_ears: bool,
    pub held_on_head: ItemStack,
}

impl AvatarRenderState {
    pub fn from_player(player: &LocalPlayer, game_type: GameType, attack_time: f32) -> Self {
        let humanoid = HumanoidRenderState::from_player(player);
        Self {
            is_spectator: game_type == GameType::Spectator,
            skin: player.get_skin(),
            cape_flap: 0.0,
            cape_lean: 0.0,
            cape_lean2: 0.0,
            arrow_count: 0,
            stinger_count: 0,
            show_hat: player.is_model_part_shown(1 << 6),
            show_jacket: player.is_model_part_shown(1 << 1),
            show_left_pants: player.is_model_part_shown(1 << 4),
            show_right_pants: player.is_model_part_shown(1 << 5),
            show_left_sleeve: player.is_model_part_shown(1 << 2),
            show_right_sleeve: player.is_model_part_shown(1 << 3),
            show_cape: player.is_model_part_shown(1 << 0),
            fall_flying_time_in_ticks: 0.0,
            should_apply_flying_y_rot: false,
            flying_y_rot: 0.0,
            parrot_on_left_shoulder: player.get_parrot_variant_on_shoulder(true),
            parrot_on_right_shoulder: player.get_parrot_variant_on_shoulder(false),
            id: 0,
            show_extra_ears: player.show_extra_ears(),
            held_on_head: ItemStack::empty(),
            humanoid: HumanoidRenderState {
                attack_time,
                ..humanoid
            },
        }
    }

    #[allow(dead_code)]
    pub fn fall_flying_scale(&self) -> f32 {
        (self.fall_flying_time_in_ticks * self.fall_flying_time_in_ticks / 100.0).clamp(0.0, 1.0)
    }
}

impl Deref for AvatarRenderState {
    type Target = HumanoidRenderState;

    fn deref(&self) -> &Self::Target {
        &self.humanoid
    }
}

impl DerefMut for AvatarRenderState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.humanoid
    }
}
