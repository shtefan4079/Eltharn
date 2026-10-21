use crate::gameplay::player::LocalPlayer;
use el_core::item::ItemStack;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HumanoidArm {
    Left,
    #[default]
    Right,
}
impl HumanoidArm {
    pub const fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum InteractionHand {
    #[default]
    MainHand,
    OffHand,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SwingAnimationType {
    None,
    #[default]
    Whack,
    Stab,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ArmPose {
    #[default]
    Empty,
    Item,
    Block,
    BowAndArrow,
    ThrowTrident,
    CrossbowCharge,
    CrossbowHold,
    Spyglass,
    TootHorn,
    Brush,
    Spear,
}
impl ArmPose {
    pub const fn is_two_handed(self) -> bool {
        matches!(
            self,
            Self::BowAndArrow | Self::CrossbowCharge | Self::CrossbowHold
        )
    }
    pub const fn affects_offhand_pose(self) -> bool {
        matches!(
            self,
            Self::BowAndArrow
                | Self::ThrowTrident
                | Self::CrossbowCharge
                | Self::CrossbowHold
                | Self::Spear
        )
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct HumanoidRenderState {
    pub body_rot: f32,
    pub x_rot: f32,
    pub y_rot: f32,
    pub walk_animation_pos: f32,
    pub walk_animation_speed: f32,
    pub age_in_ticks: f32,
    pub age_scale: f32,
    pub attack_time: f32,
    pub ticks_since_kinetic_hit_feedback: f32,
    pub main_arm: HumanoidArm,
    pub attack_arm: HumanoidArm,
    pub right_arm_pose: ArmPose,
    pub left_arm_pose: ArmPose,
    pub right_hand_item_stack: ItemStack,
    pub left_hand_item_stack: ItemStack,
    pub swing_animation_type: SwingAnimationType,
    pub swim_amount: f32,
    pub speed_value: f32,
    pub max_crossbow_charge_duration: f32,
    pub ticks_using_item: f32,
    pub use_item_hand: InteractionHand,
    pub is_crouching: bool,
    pub is_fall_flying: bool,
    pub is_visually_swimming: bool,
    pub is_passenger: bool,
    pub is_using_item: bool,
    pub elytra_rot_x: f32,
    pub elytra_rot_y: f32,
    pub elytra_rot_z: f32,
    pub head_equipment: ItemStack,
    pub chest_equipment: ItemStack,
    pub legs_equipment: ItemStack,
    pub feet_equipment: ItemStack,
}
impl Default for HumanoidRenderState {
    fn default() -> Self {
        Self {
            body_rot: 0.0,
            x_rot: 0.0,
            y_rot: 0.0,
            walk_animation_pos: 0.0,
            walk_animation_speed: 0.0,
            age_in_ticks: 0.0,
            age_scale: 1.0,
            attack_time: 0.0,
            ticks_since_kinetic_hit_feedback: 0.0,
            main_arm: HumanoidArm::Right,
            attack_arm: HumanoidArm::Right,
            right_arm_pose: ArmPose::Empty,
            left_arm_pose: ArmPose::Empty,
            right_hand_item_stack: ItemStack::empty(),
            left_hand_item_stack: ItemStack::empty(),
            swing_animation_type: SwingAnimationType::Whack,
            swim_amount: 0.0,
            speed_value: 1.0,
            max_crossbow_charge_duration: 0.0,
            ticks_using_item: 0.0,
            use_item_hand: InteractionHand::MainHand,
            is_crouching: false,
            is_fall_flying: false,
            is_visually_swimming: false,
            is_passenger: false,
            is_using_item: false,
            elytra_rot_x: 0.0,
            elytra_rot_y: 0.0,
            elytra_rot_z: 0.0,
            head_equipment: ItemStack::empty(),
            chest_equipment: ItemStack::empty(),
            legs_equipment: ItemStack::empty(),
            feet_equipment: ItemStack::empty(),
        }
    }
}
impl HumanoidRenderState {
    pub fn from_player(player: &LocalPlayer) -> Self {
        Self {
            x_rot: player.pitch.to_degrees(),
            y_rot: wrap_radians(player.head_yaw - player.body_yaw).to_degrees(),
            body_rot: player.body_yaw.to_degrees(),
            walk_animation_pos: player.walk_animation_pos,
            walk_animation_speed: player.walk_animation_speed,
            age_in_ticks: player.age_in_ticks,
            is_crouching: player.is_crouching_like_java(),
            swim_amount: player.swim_amount,
            is_visually_swimming: player.is_visually_swimming(false),
            ..Self::default()
        }
    }
    #[allow(dead_code)]
    pub fn ticks_using_item(&self, arm: HumanoidArm) -> f32 {
        let main_hand_used = self.use_item_hand == InteractionHand::MainHand;
        let is_main_arm = arm == self.main_arm;
        if self.is_using_item && main_hand_used == is_main_arm {
            self.ticks_using_item
        } else {
            0.0
        }
    }
    #[allow(dead_code)]
    pub fn item_stack_for_arm(&self, arm: HumanoidArm) -> &ItemStack {
        match arm {
            HumanoidArm::Right => &self.right_hand_item_stack,
            HumanoidArm::Left => &self.left_hand_item_stack,
        }
    }
}
fn wrap_radians(value: f32) -> f32 {
    (value + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI
}
