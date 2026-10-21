use crate::gameplay::player::LocalPlayer;
use crate::render::entity::state::{
    ArmPose, HumanoidArm, HumanoidRenderState, InteractionHand, SwingAnimationType,
};
use crate::render::item_in_hand_renderer::{ItemUseAnimation, is_charged_crossbow};
use el_core::item::ItemStack;
pub struct HumanoidMobRenderer;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmPoseContext {
    pub swinging: bool,
    pub swing_animation_type: SwingAnimationType,
    pub is_using_item: bool,
    pub used_hand: InteractionHand,
    pub use_remaining_ticks: f32,
    pub use_animation: Option<ItemUseAnimation>,
}
#[allow(dead_code)]
impl ArmPoseContext {
    pub const fn idle() -> Self {
        Self {
            swinging: false,
            swing_animation_type: SwingAnimationType::Whack,
            is_using_item: false,
            used_hand: InteractionHand::MainHand,
            use_remaining_ticks: 0.0,
            use_animation: None,
        }
    }
}
#[allow(dead_code)]
impl HumanoidMobRenderer {
    pub fn extract_humanoid_render_state(player: &LocalPlayer) -> HumanoidRenderState {
        HumanoidRenderState::from_player(player)
    }
    pub fn get_arm_pose(
        item_held_by_arm: &ItemStack,
        swinging: bool,
        swing_animation_type: SwingAnimationType,
    ) -> ArmPose {
        if swinging && swing_animation_type == SwingAnimationType::Stab {
            ArmPose::Spear
        } else if is_spear_item(item_held_by_arm) {
            ArmPose::Spear
        } else if item_held_by_arm.is_empty() {
            ArmPose::Empty
        } else {
            ArmPose::Item
        }
    }
    pub fn get_avatar_arm_pose_for_hand(
        item_in_hand: &ItemStack,
        hand: InteractionHand,
        context: ArmPoseContext,
    ) -> ArmPose {
        if item_in_hand.is_empty() {
            return ArmPose::Empty;
        }
        if !context.swinging && is_charged_crossbow(item_in_hand) {
            return ArmPose::CrossbowHold;
        }
        if context.is_using_item && context.used_hand == hand && context.use_remaining_ticks > 0.0 {
            let use_animation = context.use_animation.or_else(|| {
                ItemUseAnimation::from_stack(item_in_hand).map(|(animation, _)| animation)
            });
            match use_animation {
                Some(ItemUseAnimation::Block) => return ArmPose::Block,
                Some(ItemUseAnimation::Bow) => return ArmPose::BowAndArrow,
                Some(ItemUseAnimation::Trident) => return ArmPose::ThrowTrident,
                Some(ItemUseAnimation::Crossbow) => return ArmPose::CrossbowCharge,
                Some(ItemUseAnimation::Spyglass) => return ArmPose::Spyglass,
                Some(ItemUseAnimation::TootHorn) => return ArmPose::TootHorn,
                Some(ItemUseAnimation::Brush) => return ArmPose::Brush,
                Some(ItemUseAnimation::Spear) => return ArmPose::Spear,
                _ => {}
            }
        }
        if context.swinging && context.swing_animation_type == SwingAnimationType::Stab {
            ArmPose::Spear
        } else if is_spear_item(item_in_hand) {
            ArmPose::Spear
        } else {
            ArmPose::Item
        }
    }
    pub fn get_avatar_arm_poses(
        main_hand_item: &ItemStack,
        off_hand_item: &ItemStack,
        main_arm: HumanoidArm,
        context: ArmPoseContext,
    ) -> (ArmPose, ArmPose) {
        let main_hand_pose =
            Self::get_avatar_arm_pose_for_hand(main_hand_item, InteractionHand::MainHand, context);
        let mut off_hand_pose =
            Self::get_avatar_arm_pose_for_hand(off_hand_item, InteractionHand::OffHand, context);
        if main_hand_pose.is_two_handed() {
            off_hand_pose = if off_hand_item.is_empty() {
                ArmPose::Empty
            } else {
                ArmPose::Item
            };
        }
        if main_arm == HumanoidArm::Right {
            (main_hand_pose, off_hand_pose)
        } else {
            (off_hand_pose, main_hand_pose)
        }
    }
    pub fn get_attack_arm(main_arm: HumanoidArm, swinging_arm: InteractionHand) -> HumanoidArm {
        if swinging_arm == InteractionHand::OffHand {
            main_arm.opposite()
        } else {
            main_arm
        }
    }
    pub fn fall_flying_speed_value(delta_movement_length_sqr: f32) -> f32 {
        let speed = delta_movement_length_sqr / 0.2;
        (speed * speed * speed).max(1.0)
    }
    pub fn apply_living_flags(
        state: &mut HumanoidRenderState,
        is_crouching: bool,
        is_fall_flying: bool,
        is_visually_swimming: bool,
        is_passenger: bool,
        delta_movement_length_sqr: f32,
        swim_amount: f32,
    ) {
        state.is_crouching = is_crouching;
        state.is_fall_flying = is_fall_flying;
        state.is_visually_swimming = is_visually_swimming;
        state.is_passenger = is_passenger;
        state.speed_value = if is_fall_flying {
            Self::fall_flying_speed_value(delta_movement_length_sqr)
        } else {
            1.0
        };
        state.swim_amount = swim_amount;
    }
}
fn is_spear_item(item_stack: &ItemStack) -> bool {
    item_stack
        .components
        .get("minecraft:kinetic_weapon")
        .is_some()
}
