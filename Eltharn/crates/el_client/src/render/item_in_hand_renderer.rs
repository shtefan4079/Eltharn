#![allow(dead_code)]
use crate::gameplay::player::LocalPlayer;
use crate::render::model::player::player_model::HandSide;
use el_core::item::{DataComponent, ItemStack, ItemSwingAnimationType, KineticWeapon};
use el_nbt::Tag;
use glam::{Mat4, Vec3};
const ITEM_HEIGHT_SCALE: f32 = -0.6;
const ITEM_POS_X: f32 = 0.56;
const ITEM_POS_Y: f32 = -0.52;
const ITEM_POS_Z: f32 = -0.72;
const ITEM_SWING_X_POS_SCALE: f32 = -0.4;
const ITEM_SWING_Y_POS_SCALE: f32 = 0.2;
const ITEM_SWING_Z_POS_SCALE: f32 = -0.2;
const ITEM_PRESWING_ROT_Y: f32 = 45.0;
const ITEM_SWING_X_ROT_AMOUNT: f32 = -80.0;
const ITEM_SWING_Y_ROT_AMOUNT: f32 = -20.0;
const ITEM_SWING_Z_ROT_AMOUNT: f32 = -20.0;
const EAT_JIGGLE_EXPONENT: f32 = 27.0;
const EAT_EXTRA_JIGGLE_CUTOFF: f32 = 0.8;
const BOW_CHARGE_FULL_TIME: f32 = 20.0;
const TRIDENT_CHARGE_FULL_TIME: f32 = 10.0;
const CROSSBOW_MIN_SHAKE_CHARGE: f32 = 0.1;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemUseAnimation {
    None,
    Eat,
    Drink,
    Block,
    Bow,
    Crossbow,
    Trident,
    Spyglass,
    TootHorn,
    Brush,
    Bundle,
    Spear,
}
impl ItemUseAnimation {
    pub fn vanilla_name(self) -> &'static str {
        match self {
            Self::None => "NONE",
            Self::Eat => "EAT",
            Self::Drink => "DRINK",
            Self::Block => "BLOCK",
            Self::Bow => "BOW",
            Self::Crossbow => "CROSSBOW",
            Self::Trident => "TRIDENT",
            Self::Spyglass => "SPYGLASS",
            Self::TootHorn => "TOOT_HORN",
            Self::Brush => "BRUSH",
            Self::Bundle => "BUNDLE",
            Self::Spear => "SPEAR",
        }
    }
    pub const fn has_custom_arm_transform(self) -> bool {
        matches!(self, Self::Eat | Self::Drink | Self::Spear)
    }
    pub fn from_stack(item_stack: &ItemStack) -> Option<(Self, f32)> {
        if item_stack.is_empty() {
            return None;
        }
        if item_stack.is_vanilla_bow_item() {
            return Some((Self::Bow, 72000.0));
        }
        let path = item_stack.item.location.path.as_str();
        if item_stack.components.get("minecraft:consumable").is_some() {
            let consumable = item_stack.consumable_properties()?;
            let animation = Self::from_consumable_animation_name(&consumable.animation)?;
            return Some((animation, consumable.consume_ticks() as f32));
        }
        if item_stack
            .components
            .get("minecraft:blocks_attacks")
            .is_some()
        {
            return Some((Self::Block, 72000.0));
        }
        if item_stack
            .components
            .get("minecraft:kinetic_weapon")
            .is_some()
        {
            return Some((Self::Spear, 72000.0));
        }
        match path {
            "crossbow" => Some((Self::Crossbow, 72000.0)),
            "trident" => Some((Self::Trident, 72000.0)),
            "spyglass" => Some((Self::Spyglass, 1200.0)),
            "goat_horn" if item_stack.components.get("minecraft:instrument").is_some() => {
                Some((Self::TootHorn, 140.0))
            }
            "brush" => Some((Self::Brush, 72000.0)),
            "bundle" | "white_bundle" | "orange_bundle" | "magenta_bundle"
            | "light_blue_bundle" | "yellow_bundle" | "lime_bundle" | "pink_bundle"
            | "gray_bundle" | "light_gray_bundle" | "cyan_bundle" | "purple_bundle"
            | "blue_bundle" | "brown_bundle" | "green_bundle" | "red_bundle" | "black_bundle" => {
                Some((Self::Bundle, 72000.0))
            }
            _ => None,
        }
    }
    fn from_consumable_animation_name(name: &str) -> Option<Self> {
        match name {
            "none" => Some(Self::None),
            "eat" => Some(Self::Eat),
            "drink" => Some(Self::Drink),
            "block" | "block_kind" => Some(Self::Block),
            "bow" => Some(Self::Bow),
            "trident" => Some(Self::Trident),
            "crossbow" => Some(Self::Crossbow),
            "spyglass" => Some(Self::Spyglass),
            "toot_horn" => Some(Self::TootHorn),
            "brush" => Some(Self::Brush),
            "bundle" => Some(Self::Bundle),
            "spear" => Some(Self::Spear),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HandRenderSelection {
    pub render_main_hand: bool,
    pub render_off_hand: bool,
}
impl HandRenderSelection {
    pub const RENDER_BOTH_HANDS: Self = Self {
        render_main_hand: true,
        render_off_hand: true,
    };
    pub const RENDER_MAIN_HAND_ONLY: Self = Self {
        render_main_hand: true,
        render_off_hand: false,
    };
    pub const RENDER_OFF_HAND_ONLY: Self = Self {
        render_main_hand: false,
        render_off_hand: true,
    };
    pub fn only_for_hand(hand: HandSide) -> Self {
        match hand {
            HandSide::Right => Self::RENDER_MAIN_HAND_ONLY,
            HandSide::Left => Self::RENDER_OFF_HAND_ONLY,
        }
    }
}
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ItemInHandRendererState {
    pub main_hand_item: Option<String>,
    pub off_hand_item: Option<String>,
    pub main_hand_height: f32,
    pub o_main_hand_height: f32,
    pub off_hand_height: f32,
    pub o_off_hand_height: f32,
    pub swing_time: i32,
    pub swing_duration: i32,
    pub swinging: bool,
    pub attack_anim: f32,
    pub o_attack_anim: f32,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandHeightTick {
    pub old_height: f32,
    pub height: f32,
    pub should_swap_visible_item: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandUseContext {
    pub hand: HandSide,
    pub inverse_arm_height: f32,
    pub attack_value: f32,
    pub frame_interp: f32,
    pub use_remaining_ticks: f32,
    pub use_duration: f32,
    pub charged_crossbow: bool,
    pub crossbow_charge_duration: f32,
}
impl HandUseContext {
    pub const fn new(hand: HandSide, inverse_arm_height: f32, attack_value: f32) -> Self {
        Self {
            hand,
            inverse_arm_height,
            attack_value,
            frame_interp: 0.0,
            use_remaining_ticks: 0.0,
            use_duration: 0.0,
            charged_crossbow: false,
            crossbow_charge_duration: 25.0,
        }
    }
}
impl Default for ItemInHandRendererState {
    fn default() -> Self {
        Self {
            main_hand_item: None,
            off_hand_item: None,
            main_hand_height: 1.0,
            o_main_hand_height: 1.0,
            off_hand_height: 1.0,
            o_off_hand_height: 1.0,
            swing_time: 0,
            swing_duration: 6,
            swinging: false,
            attack_anim: 0.0,
            o_attack_anim: 0.0,
        }
    }
}
#[allow(dead_code)]
impl ItemInHandRendererState {
    pub fn start_swing(&mut self) {
        if !self.swinging || self.swing_time >= self.swing_duration / 2 || self.swing_time < 0 {
            self.swing_time = -1;
            self.swinging = true;
        }
    }
    pub fn tick_attack_animation(&mut self) {
        self.o_attack_anim = self.attack_anim;
        if self.swinging {
            self.swing_time += 1;
            if self.swing_time >= self.swing_duration {
                self.swing_time = 0;
                self.swinging = false;
            }
        } else {
            self.swing_time = 0;
        }
        self.attack_anim = self.swing_time as f32 / self.swing_duration as f32;
    }
    pub fn get_attack_anim(&self, partial_tick: f32) -> f32 {
        let mut diff = self.attack_anim - self.o_attack_anim;
        if diff < 0.0 {
            diff += 1.0;
        }
        self.o_attack_anim + diff * partial_tick.clamp(0.0, 1.0)
    }
}
pub fn get_item_use_animation_and_duration(
    item_stack: &ItemStack,
) -> Option<(ItemUseAnimation, f32)> {
    ItemUseAnimation::from_stack(item_stack)
}
pub fn evaluate_which_hands_to_render(
    player: &LocalPlayer,
    active_hotbar_slot: usize,
    using_item_hand: Option<HandSide>,
) -> HandRenderSelection {
    evaluate_which_hands_to_render_with_main_hand(
        player,
        active_hotbar_slot,
        using_item_hand,
        HandSide::Right,
    )
}
pub fn evaluate_which_hands_to_render_with_main_hand(
    player: &LocalPlayer,
    active_hotbar_slot: usize,
    using_item_hand: Option<HandSide>,
    main_hand_side: HandSide,
) -> HandRenderSelection {
    let main_item = player
        .inventory
        .get(active_hotbar_slot)
        .or_else(|| player.inventory.first());
    let Some(main_item) = main_item else {
        return HandRenderSelection::RENDER_MAIN_HAND_ONLY;
    };
    let off_item = player.inventory.get(40).unwrap_or(main_item);
    let _ = main_hand_side;
    evaluate_which_hands_to_render_for_stacks(
        main_item,
        off_item,
        using_item_hand.map(|hand| hand == HandSide::Right),
        false,
    )
}
pub fn evaluate_which_hands_to_render_for_stacks(
    main_item: &ItemStack,
    off_item: &ItemStack,
    using_hand_is_main: Option<bool>,
    is_spectator: bool,
) -> HandRenderSelection {
    if is_spectator {
        return HandRenderSelection {
            render_main_hand: false,
            render_off_hand: false,
        };
    }
    if !is_bow_like(main_item) && !is_bow_like(off_item) {
        HandRenderSelection::RENDER_BOTH_HANDS
    } else if let Some(using_is_main) = using_hand_is_main {
        selection_using_item_while_holding_bow_like_for_stacks(main_item, off_item, using_is_main)
    } else if is_charged_crossbow(main_item) {
        HandRenderSelection::RENDER_MAIN_HAND_ONLY
    } else {
        HandRenderSelection::RENDER_BOTH_HANDS
    }
}
fn selection_using_item_while_holding_bow_like_for_stacks(
    main_item: &ItemStack,
    off_item: &ItemStack,
    using_hand_is_main: bool,
) -> HandRenderSelection {
    let used_item = if using_hand_is_main {
        main_item
    } else {
        off_item
    };
    if is_bow_like(used_item) {
        if using_hand_is_main {
            HandRenderSelection::RENDER_MAIN_HAND_ONLY
        } else {
            HandRenderSelection::RENDER_OFF_HAND_ONLY
        }
    } else if using_hand_is_main && is_charged_crossbow(off_item) {
        HandRenderSelection::RENDER_MAIN_HAND_ONLY
    } else {
        HandRenderSelection::RENDER_BOTH_HANDS
    }
}
pub fn is_bow_like(item_stack: &ItemStack) -> bool {
    matches!(item_stack.item.location.path.as_str(), "bow" | "crossbow")
}
pub fn is_map(item_stack: &ItemStack) -> bool {
    matches!(item_stack.item.location.path.as_str(), "filled_map" | "map")
        || item_stack.components.get("minecraft:map_id").is_some()
}
pub fn is_charged_crossbow(item_stack: &ItemStack) -> bool {
    if item_stack.item.location.path != "crossbow" {
        return false;
    }
    matches!(
        item_stack.components.get("minecraft:charged_projectiles"),
        Some(DataComponent::Generic { value : Tag::List(items), .. }) if ! items
        .is_empty()
    )
}
pub fn calculate_map_tilt(x_rot_degrees: f32) -> f32 {
    let tilt = (1.0 - x_rot_degrees / 45.0 + 0.1).clamp(0.0, 1.0);
    -(tilt * std::f32::consts::PI).cos() * 0.5 + 0.5
}
pub fn apply_item_arm_transform(hand: HandSide, inverse_arm_height: f32) -> Mat4 {
    let invert = hand_invert(hand);
    Mat4::from_translation(Vec3::new(
        invert * ITEM_POS_X,
        ITEM_POS_Y + inverse_arm_height * ITEM_HEIGHT_SCALE,
        ITEM_POS_Z,
    ))
}
pub fn apply_item_arm_attack_transform(hand: HandSide, attack_value: f32) -> Mat4 {
    let invert = hand_invert(hand);
    let sqrt_attack = attack_value.sqrt();
    let y_swing_rotation = (attack_value * attack_value * std::f32::consts::PI).sin();
    let xz_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();
    Mat4::from_rotation_y(
        (invert * (ITEM_PRESWING_ROT_Y + y_swing_rotation * ITEM_SWING_Y_ROT_AMOUNT)).to_radians(),
    ) * Mat4::from_rotation_z((invert * xz_swing_rotation * ITEM_SWING_Z_ROT_AMOUNT).to_radians())
        * Mat4::from_rotation_x((xz_swing_rotation * ITEM_SWING_X_ROT_AMOUNT).to_radians())
        * Mat4::from_rotation_y((invert * -ITEM_PRESWING_ROT_Y).to_radians())
}
pub fn swing_arm_matrix(hand: HandSide, attack_value: f32) -> Mat4 {
    let invert = hand_invert(hand);
    let sqrt_attack = attack_value.sqrt();
    let x_swing_position = ITEM_SWING_X_POS_SCALE * (sqrt_attack * std::f32::consts::PI).sin();
    let y_swing_position =
        ITEM_SWING_Y_POS_SCALE * (sqrt_attack * std::f32::consts::PI * 2.0).sin();
    let z_swing_position = ITEM_SWING_Z_POS_SCALE * (attack_value * std::f32::consts::PI).sin();
    Mat4::from_translation(Vec3::new(
        invert * x_swing_position,
        y_swing_position,
        z_swing_position,
    )) * apply_item_arm_attack_transform(hand, attack_value)
}
pub fn swing_animation_matrix(
    hand: HandSide,
    attack_value: f32,
    animation_type: ItemSwingAnimationType,
) -> Mat4 {
    match animation_type {
        ItemSwingAnimationType::None => Mat4::IDENTITY,
        ItemSwingAnimationType::Whack => swing_arm_matrix(hand, attack_value),
        ItemSwingAnimationType::Stab => spear_first_person_attack_matrix(hand, attack_value),
    }
}
pub fn spear_first_person_attack_matrix(hand: HandSide, attack_value: f32) -> Mat4 {
    let invert = hand_invert(hand);
    let (starting_amount, middle_amount, ending_amount) =
        spear_first_person_attack_amounts(attack_value);
    Mat4::from_translation(Vec3::new(
        invert * 0.1 * (starting_amount - middle_amount),
        -0.075 * (starting_amount - ending_amount),
        0.65 * (starting_amount - middle_amount),
    )) * Mat4::from_rotation_x((-70.0 * (starting_amount - ending_amount)).to_radians())
        * Mat4::from_translation(Vec3::new(0.0, 0.0, -0.25 * (ending_amount - middle_amount)))
}
fn spear_first_person_attack_amounts(attack_value: f32) -> (f32, f32, f32) {
    let starting_amount = java_ease_in_out_sine(java_progress(attack_value, 0.0, 0.05));
    let middle_amount = java_ease_out_back(java_progress(attack_value, 0.05, 0.2));
    let ending_amount = java_ease_in_out_expo(java_progress(attack_value, 0.4, 1.0));
    (starting_amount, middle_amount, ending_amount)
}
fn java_progress(value: f32, start: f32, end: f32) -> f32 {
    ((value - start) / (end - start)).clamp(0.0, 1.0)
}
fn java_ease_in_out_sine(value: f32) -> f32 {
    -((std::f32::consts::PI * value).cos() - 1.0) / 2.0
}
fn java_ease_out_back(value: f32) -> f32 {
    let delta = value - 1.0;
    1.0 + 2.70158 * delta * delta * delta + 1.70158 * delta * delta
}
fn java_ease_in_out_expo(value: f32) -> f32 {
    if value < 0.5 {
        if value == 0.0 {
            0.0
        } else {
            2.0_f32.powf(20.0 * value - 10.0) / 2.0
        }
    } else if value == 1.0 {
        1.0
    } else {
        (2.0 - 2.0_f32.powf(-20.0 * value + 10.0)) / 2.0
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpearUseAnimationParams {
    pub raise_progress: f32,
    pub raise_progress_start: f32,
    pub raise_progress_middle: f32,
    pub raise_progress_end: f32,
    pub sway_progress: f32,
    pub lower_progress: f32,
    pub raise_back_progress: f32,
    pub sway_intensity: f32,
    pub sway_scale_slow: f32,
    pub sway_scale_fast: f32,
}
pub fn spear_use_animation_params(
    kinetic_weapon: KineticWeapon,
    time_held: f32,
) -> SpearUseAnimationParams {
    let finish_raising_tick = kinetic_weapon.delay_ticks;
    let finish_swaying_tick = kinetic_weapon
        .dismount_conditions
        .map_or(0, |condition| condition.max_duration_ticks)
        .wrapping_add(finish_raising_tick);
    let start_swaying_tick = finish_swaying_tick.wrapping_sub(20);
    let finish_lowering_tick = kinetic_weapon
        .knockback_conditions
        .map_or(0, |condition| condition.max_duration_ticks)
        .wrapping_add(finish_raising_tick);
    let start_lowering_tick = finish_lowering_tick.wrapping_sub(40);
    let finish_raising_back_tick = kinetic_weapon
        .damage_conditions
        .map_or(0, |condition| condition.max_duration_ticks)
        .wrapping_add(finish_raising_tick);
    let raise_progress = java_progress(time_held, 0.0, finish_raising_tick as f32);
    let raise_progress_start = java_progress(raise_progress, 0.0, 0.5);
    let raise_progress_middle = java_progress(raise_progress, 0.5, 0.8);
    let raise_progress_end = java_progress(raise_progress, 0.8, 1.0);
    let sway_progress = java_progress(
        time_held,
        start_swaying_tick as f32,
        finish_swaying_tick as f32,
    );
    let lower_progress = java_ease_out_cubic(java_ease_in_out_elastic(java_progress(
        time_held - 20.0,
        start_lowering_tick as f32,
        finish_lowering_tick as f32,
    )));
    let raise_back_progress = java_progress(
        time_held,
        finish_raising_back_tick.wrapping_sub(5) as f32,
        finish_raising_back_tick as f32,
    );
    let sway_intensity =
        2.0 * java_ease_out_circ(sway_progress) - 2.0 * java_ease_in_circ(raise_back_progress);
    let sway_scale_slow = (time_held * 19.0_f32.to_radians()).sin() * sway_intensity;
    let sway_scale_fast = (time_held * 30.0_f32.to_radians()).sin() * sway_intensity;
    SpearUseAnimationParams {
        raise_progress,
        raise_progress_start,
        raise_progress_middle,
        raise_progress_end,
        sway_progress,
        lower_progress,
        raise_back_progress,
        sway_intensity,
        sway_scale_slow,
        sway_scale_fast,
    }
}
pub fn spear_first_person_use_matrix(
    hand: HandSide,
    time_held: f32,
    ticks_since_kinetic_hit_feedback: f32,
    kinetic_weapon: KineticWeapon,
) -> Mat4 {
    let invert = hand_invert(hand);
    let params = spear_use_animation_params(kinetic_weapon, time_held);
    let x_rotation = -65.0 * java_ease_in_out_back(params.raise_progress)
        - 35.0 * params.lower_progress
        + 100.0 * params.raise_back_progress
        - 0.5 * params.sway_scale_fast;
    let y_rotation_argument = invert
        * (-90.0 * java_progress(params.raise_progress, 0.5, 0.55)
            + 90.0 * params.sway_progress
            + 2.0 * params.sway_scale_slow);
    Mat4::from_translation(Vec3::new(
        invert
            * (params.raise_progress * 0.15
                - params.raise_progress_end * 0.05
                - params.sway_progress * 0.1
                + params.sway_scale_slow * 0.005),
        -params.raise_progress * 0.075
            + params.raise_progress_middle * 0.075
            + params.sway_scale_fast * 0.01,
        params.raise_progress_start * 0.05 - params.raise_progress_end * 0.05
            + params.sway_scale_slow * 0.005,
    )) * rotate_around_matrix(
        Mat4::from_rotation_x(x_rotation.to_radians()),
        Vec3::new(0.0, 0.1, 0.0),
    ) * rotate_around_matrix(
        Mat4::from_rotation_y((-y_rotation_argument).to_radians()),
        Vec3::new(invert * 0.15, 0.0, 0.0),
    ) * Mat4::from_translation(Vec3::new(
        0.0,
        -kinetic_hit_feedback_amount(ticks_since_kinetic_hit_feedback),
        0.0,
    ))
}
fn rotate_around_matrix(rotation: Mat4, pivot: Vec3) -> Mat4 {
    Mat4::from_translation(pivot) * rotation * Mat4::from_translation(-pivot)
}
fn kinetic_hit_feedback_amount(ticks_since_feedback_start: f32) -> f32 {
    0.4 * (java_ease_out_quart(java_progress(ticks_since_feedback_start, 1.0, 3.0))
        - java_ease_in_out_sine(java_progress(ticks_since_feedback_start, 3.0, 10.0)))
}
fn java_ease_in_out_back(value: f32) -> f32 {
    if value < 0.5 {
        4.0 * value * value * (7.189_819 * value - 2.594_909_4) / 2.0
    } else {
        let delta = 2.0 * value - 2.0;
        (delta * delta * (3.594_909_4 * delta + 2.594_909_4) + 2.0) / 2.0
    }
}
fn java_ease_in_out_elastic(value: f32) -> f32 {
    if value == 0.0 {
        0.0
    } else if value == 1.0 {
        1.0
    } else {
        let sine = ((20.0 * value - 11.125) * std::f32::consts::PI * 4.0 / 9.0).sin();
        if value < 0.5 {
            -(2.0_f32.powf(20.0 * value - 10.0) * sine) / 2.0
        } else {
            2.0_f32.powf(-20.0 * value + 10.0) * sine / 2.0 + 1.0
        }
    }
}
fn java_ease_out_cubic(value: f32) -> f32 {
    1.0 - (1.0 - value).powi(3)
}
fn java_ease_out_circ(value: f32) -> f32 {
    (1.0 - (value - 1.0).powi(2)).sqrt()
}
fn java_ease_in_circ(value: f32) -> f32 {
    -(1.0 - value * value).sqrt() + 1.0
}
fn java_ease_out_quart(value: f32) -> f32 {
    1.0 - (1.0 - value).powi(4)
}
pub fn one_handed_map_item_matrix(
    hand: HandSide,
    inverse_arm_height: f32,
    attack_value: f32,
) -> Mat4 {
    let invert = hand_invert(hand);
    let sqrt_attack_value = attack_value.sqrt();
    let x_swing = (sqrt_attack_value * std::f32::consts::PI).sin();
    let x_swing_position = -0.5 * x_swing;
    let y_swing_position = 0.4 * (sqrt_attack_value * std::f32::consts::PI * 2.0).sin();
    let z_swing_position = -0.3 * (attack_value * std::f32::consts::PI).sin();
    Mat4::from_translation(Vec3::new(invert * 0.125, -0.125, 0.0))
        * Mat4::from_translation(Vec3::new(
            invert * 0.51,
            -0.08 + inverse_arm_height * -1.2,
            -0.75,
        ))
        * Mat4::from_translation(Vec3::new(
            invert * x_swing_position,
            y_swing_position - 0.3 * x_swing,
            z_swing_position,
        ))
        * Mat4::from_rotation_x((x_swing * -45.0_f32).to_radians())
        * Mat4::from_rotation_y((invert * x_swing * -30.0_f32).to_radians())
}
pub fn two_handed_map_base_matrix(
    x_rot_degrees: f32,
    inverse_arm_height: f32,
    attack_value: f32,
) -> Mat4 {
    let sqrt_attack_value = attack_value.sqrt();
    let y_swing_position = -0.2 * (attack_value * std::f32::consts::PI).sin();
    let z_swing_position = -0.4 * (sqrt_attack_value * std::f32::consts::PI).sin();
    let map_tilt = calculate_map_tilt(x_rot_degrees);
    Mat4::from_translation(Vec3::new(0.0, -y_swing_position / 2.0, z_swing_position))
        * Mat4::from_translation(Vec3::new(
            0.0,
            0.04 + inverse_arm_height * -1.2 + map_tilt * -0.5,
            -0.72,
        ))
        * Mat4::from_rotation_x((map_tilt * -85.0_f32).to_radians())
}
pub fn map_model_matrix(map_base_matrix: Mat4) -> Mat4 {
    map_base_matrix
        * Mat4::from_rotation_y(180.0_f32.to_radians())
        * Mat4::from_rotation_z(180.0_f32.to_radians())
        * Mat4::from_scale(Vec3::splat(0.38))
        * Mat4::from_translation(Vec3::new(-0.5, -0.5, 0.0))
        * Mat4::from_scale(Vec3::splat(0.0078125))
}
pub fn map_hand_matrix(arm: HandSide) -> Mat4 {
    let invert = hand_invert(arm);
    Mat4::from_rotation_y(92.0_f32.to_radians())
        * Mat4::from_rotation_x(45.0_f32.to_radians())
        * Mat4::from_rotation_z((invert * -41.0_f32).to_radians())
        * Mat4::from_translation(Vec3::new(invert * 0.3, -1.1, 0.45))
}
pub fn eat_transform_matrix(
    hand: HandSide,
    frame_interp: f32,
    use_remaining_ticks: f32,
    use_duration: f32,
) -> Mat4 {
    let curr_usage_time = (use_remaining_ticks - frame_interp + 1.0).max(0.0);
    let scaled_usage_time = if use_duration > 0.0 {
        curr_usage_time / use_duration
    } else {
        0.0
    };
    let mut matrix = Mat4::IDENTITY;
    if scaled_usage_time < EAT_EXTRA_JIGGLE_CUTOFF {
        let extra_height_offset = (curr_usage_time / 4.0 * std::f32::consts::PI).cos().abs() * 0.1;
        matrix *= Mat4::from_translation(Vec3::new(0.0, extra_height_offset, 0.0));
    }
    let eat_jiggle = 1.0 - scaled_usage_time.clamp(0.0, 1.0).powf(EAT_JIGGLE_EXPONENT);
    let invert = hand_invert(hand);
    matrix
        * Mat4::from_translation(Vec3::new(eat_jiggle * 0.6 * invert, eat_jiggle * -0.5, 0.0))
        * Mat4::from_rotation_y((invert * eat_jiggle * 90.0_f32).to_radians())
        * Mat4::from_rotation_x((eat_jiggle * 10.0_f32).to_radians())
        * Mat4::from_rotation_z((invert * eat_jiggle * 30.0_f32).to_radians())
}
pub fn brush_transform_matrix(hand: HandSide, frame_interp: f32, use_remaining_ticks: f32) -> Mat4 {
    let brush_animation_remaining_ticks = (use_remaining_ticks as i32 % 10) as f32;
    let delta_since_last_update = brush_animation_remaining_ticks - frame_interp + 1.0;
    let scaled_usage_time = 1.0 - delta_since_last_update / 10.0;
    let current_swipe_angle = -15.0 + 75.0 * (scaled_usage_time * 2.0 * std::f32::consts::PI).cos();
    match hand {
        HandSide::Left => {
            Mat4::from_translation(Vec3::new(0.1, 0.83, 0.35))
                * Mat4::from_rotation_x(-80.0_f32.to_radians())
                * Mat4::from_rotation_y(-90.0_f32.to_radians())
                * Mat4::from_rotation_x(current_swipe_angle.to_radians())
                * Mat4::from_translation(Vec3::new(-0.3, 0.22, 0.35))
        }
        HandSide::Right => {
            Mat4::from_translation(Vec3::new(-0.25, 0.22, 0.35))
                * Mat4::from_rotation_x(-80.0_f32.to_radians())
                * Mat4::from_rotation_y(90.0_f32.to_radians())
                * Mat4::from_rotation_x(current_swipe_angle.to_radians())
        }
    }
}
pub fn bow_use_transform_matrix(
    hand: HandSide,
    frame_interp: f32,
    use_remaining_ticks: f32,
    use_duration: f32,
) -> Mat4 {
    let invert = hand_invert(hand);
    let time_held = use_duration - (use_remaining_ticks - frame_interp + 1.0);
    let mut power = time_held / BOW_CHARGE_FULL_TIME;
    power = ((power * power + power * 2.0) / 3.0).min(1.0);
    charge_transform_matrix(
        invert,
        Vec3::new(invert * -0.2785682, 0.18344387, 0.15731531),
        Vec3::new(-13.935, invert * 35.3, invert * -9.785),
        power,
        time_held,
        0.04,
        0.2,
    )
}
pub fn trident_use_transform_matrix(
    hand: HandSide,
    frame_interp: f32,
    use_remaining_ticks: f32,
    use_duration: f32,
) -> Mat4 {
    let invert = hand_invert(hand);
    let time_held = use_duration - (use_remaining_ticks - frame_interp + 1.0);
    let power = (time_held / TRIDENT_CHARGE_FULL_TIME).min(1.0);
    charge_transform_matrix(
        invert,
        Vec3::new(invert * -0.5, 0.7, 0.1),
        Vec3::new(-55.0, invert * 35.3, invert * -9.785),
        power,
        time_held,
        0.2,
        0.2,
    )
}
pub fn crossbow_use_transform_matrix(
    hand: HandSide,
    frame_interp: f32,
    use_remaining_ticks: f32,
    use_duration: f32,
    charge_duration: f32,
) -> Mat4 {
    let invert = hand_invert(hand);
    let time_held = use_duration - (use_remaining_ticks - frame_interp + 1.0);
    let charge_duration = charge_duration.max(1.0);
    let power = (time_held / charge_duration).min(1.0);
    charge_transform_matrix(
        invert,
        Vec3::new(invert * -0.4785682, -0.094387, 0.05731531),
        Vec3::new(-11.935, invert * 65.3, invert * -9.785),
        power,
        time_held,
        0.04,
        0.2,
    )
}
pub fn charged_crossbow_hold_matrix(hand: HandSide, is_main_hand: bool, attack_value: f32) -> Mat4 {
    if is_main_hand && attack_value < 0.001 {
        let invert = hand_invert(hand);
        Mat4::from_translation(Vec3::new(invert * -0.641864, 0.0, 0.0))
            * Mat4::from_rotation_y((invert * 10.0_f32).to_radians())
    } else {
        Mat4::IDENTITY
    }
}
pub fn auto_spin_attack_matrix(hand: HandSide) -> Mat4 {
    let invert = hand_invert(hand);
    Mat4::from_translation(Vec3::new(invert * -0.4, 0.8, 0.3))
        * Mat4::from_rotation_y((invert * 65.0_f32).to_radians())
        * Mat4::from_rotation_z((invert * -85.0_f32).to_radians())
}
pub fn item_use_transform_matrix(
    ctx: HandUseContext,
    item_stack: &ItemStack,
    use_animation: ItemUseAnimation,
) -> Mat4 {
    let base = if use_animation.has_custom_arm_transform() {
        Mat4::IDENTITY
    } else {
        apply_item_arm_transform(ctx.hand, ctx.inverse_arm_height)
    };
    match use_animation {
        ItemUseAnimation::None => base,
        ItemUseAnimation::Eat | ItemUseAnimation::Drink => {
            eat_transform_matrix(
                ctx.hand,
                ctx.frame_interp,
                ctx.use_remaining_ticks,
                ctx.use_duration,
            ) * apply_item_arm_transform(ctx.hand, ctx.inverse_arm_height)
        }
        ItemUseAnimation::Block => block_use_transform_matrix(
            ctx.hand,
            ctx.inverse_arm_height,
            item_stack.is_vanilla_shield_item(),
        ),
        ItemUseAnimation::Bow => {
            base * bow_use_transform_matrix(
                ctx.hand,
                ctx.frame_interp,
                ctx.use_remaining_ticks,
                ctx.use_duration,
            )
        }
        ItemUseAnimation::Crossbow => {
            base * crossbow_use_transform_matrix(
                ctx.hand,
                ctx.frame_interp,
                ctx.use_remaining_ticks,
                ctx.use_duration,
                ctx.crossbow_charge_duration,
            )
        }
        ItemUseAnimation::Trident => {
            base * trident_use_transform_matrix(
                ctx.hand,
                ctx.frame_interp,
                ctx.use_remaining_ticks,
                ctx.use_duration,
            )
        }
        ItemUseAnimation::Spyglass | ItemUseAnimation::TootHorn => base,
        ItemUseAnimation::Brush => {
            base * brush_transform_matrix(ctx.hand, ctx.frame_interp, ctx.use_remaining_ticks)
        }
        ItemUseAnimation::Bundle => base * swing_arm_matrix(ctx.hand, ctx.attack_value),
        ItemUseAnimation::Spear => apply_item_arm_transform(ctx.hand, 0.0),
    }
}
pub fn block_use_transform_matrix(
    hand: HandSide,
    inverse_arm_height: f32,
    is_vanilla_shield_item: bool,
) -> Mat4 {
    let base = apply_item_arm_transform(hand, inverse_arm_height);
    if is_vanilla_shield_item {
        return base;
    }
    let invert = hand_invert(hand);
    base * Mat4::from_translation(Vec3::new(invert * -0.14142136, 0.08, 0.14142136))
        * Mat4::from_rotation_x(-102.25_f32.to_radians())
        * Mat4::from_rotation_y((invert * 13.365_f32).to_radians())
        * Mat4::from_rotation_z((invert * 78.05_f32).to_radians())
}
fn charge_transform_matrix(
    invert: f32,
    translation: Vec3,
    rotations_degrees: Vec3,
    power: f32,
    time_held: f32,
    z_translation_scale: f32,
    z_scale: f32,
) -> Mat4 {
    let mut matrix = Mat4::from_translation(translation)
        * Mat4::from_rotation_x(rotations_degrees.x.to_radians())
        * Mat4::from_rotation_y(rotations_degrees.y.to_radians())
        * Mat4::from_rotation_z(rotations_degrees.z.to_radians());
    if power > CROSSBOW_MIN_SHAKE_CHARGE {
        let shake_offset = ((time_held - CROSSBOW_MIN_SHAKE_CHARGE) * 1.3).sin();
        let shake = shake_offset * (power - CROSSBOW_MIN_SHAKE_CHARGE);
        matrix *= Mat4::from_translation(Vec3::new(0.0, shake * 0.004, 0.0));
    }
    matrix
        * Mat4::from_translation(Vec3::new(0.0, 0.0, power * z_translation_scale))
        * Mat4::from_scale(Vec3::new(1.0, 1.0, 1.0 + power * z_scale))
        * Mat4::from_rotation_y((invert * -45.0_f32).to_radians())
}
fn hand_invert(hand: HandSide) -> f32 {
    match hand {
        HandSide::Right => 1.0,
        HandSide::Left => -1.0,
    }
}
