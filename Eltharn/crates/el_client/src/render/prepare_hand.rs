use el_core::item::ItemStack;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandItemProperties {
    pub hand_animation_on_swap: bool,
    pub swap_animation_scale: f32,
}
impl Default for HandItemProperties {
    fn default() -> Self {
        Self {
            hand_animation_on_swap: true,
            swap_animation_scale: 1.0,
        }
    }
}
impl HandItemProperties {
    pub fn for_stack(
        stack: Option<&ItemStack>,
        definitions: &std::collections::HashMap<
            el_core::resource_location::ResourceLocation,
            serde_json::Value,
        >,
    ) -> Self {
        let Some(stack) = stack.filter(|stack| !stack.is_empty()) else {
            return Self::default();
        };
        let model = match stack.components.get("minecraft:item_model") {
            Some(el_core::item::DataComponent::Generic {
                value: el_nbt::Tag::String(id),
                ..
            }) => el_core::resource_location::ResourceLocation::from_string(id),
            Some(_) => return Self::default(),
            None if stack.components.patch.map.get("minecraft:item_model") == Some(&None) => {
                return Self::default();
            }
            None => stack.item.location.clone(),
        };
        let Some(definition) = definitions.get(&model) else {
            return Self::default();
        };
        Self {
            hand_animation_on_swap: definition
                .get("hand_animation_on_swap")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            swap_animation_scale: definition
                .get("swap_animation_scale")
                .and_then(serde_json::Value::as_f64)
                .map(|value| value as f32)
                .unwrap_or(1.0),
        }
    }
}
pub const HAND_HEIGHT_STEP: f32 = 0.4;
pub const HAND_HEIGHT_MIN: f32 = 0.0;
pub const HAND_HEIGHT_MAX: f32 = 1.0;
pub const HAND_SWAP_THRESHOLD: f32 = 0.1;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldHands {
    Both,
    MainOnly,
    OffOnly,
    None,
}
pub(crate) fn step_toward(current: f32, target: f32) -> f32 {
    let delta = (target - current).clamp(-HAND_HEIGHT_STEP, HAND_HEIGHT_STEP);
    (current + delta).clamp(HAND_HEIGHT_MIN, HAND_HEIGHT_MAX)
}
pub fn should_instantly_replace_visible_item(
    current: &Option<ItemStack>,
    next: &Option<ItemStack>,
    play_swap_animation: bool,
) -> bool {
    !play_swap_animation
        || match (current, next) {
            (None, None) => true,
            (Some(current), Some(next)) => {
                current.item.location == next.item.location
                    && current.count == next.count
                    && current.components.merged.len() == next.components.merged.len()
                    && current.components.merged.iter().all(|(key, value)| {
                        next.components.merged.get(key).is_some_and(|next_value| {
                            key == "minecraft:damage" || value == next_value
                        })
                    })
            }
            _ => false,
        }
}
#[allow(clippy::too_many_arguments)]
pub fn hand_changed(
    visible_texture: &Option<String>,
    visible_stack: &Option<ItemStack>,
    visible_custom_texture_key: &Option<String>,
    visible_using_item_context: bool,
    visible_pull_stage: u8,
    next_texture: &Option<String>,
    next_stack: &Option<ItemStack>,
    next_custom_texture_key: &Option<String>,
    next_using_item_context: bool,
    next_pull_stage: u8,
) -> bool {
    visible_texture != next_texture
        || visible_stack != next_stack
        || visible_custom_texture_key != next_custom_texture_key
        || visible_using_item_context != next_using_item_context
        || visible_pull_stage != next_pull_stage
}
pub fn hand_height_target(
    visible_texture: &Option<String>,
    visible_stack: &Option<ItemStack>,
    next_texture: &Option<String>,
    next_stack: &Option<ItemStack>,
    raised_height: f32,
) -> f32 {
    if visible_texture == next_texture && visible_stack == next_stack {
        raised_height
    } else {
        0.0
    }
}
pub fn should_swap_visible_item(height: f32, changed: bool) -> bool {
    changed && height < HAND_SWAP_THRESHOLD
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandTickPlan {
    pub changed: bool,
    pub instant_replace: bool,
    pub target: f32,
    pub new_height: f32,
    pub swap: bool,
}
#[allow(clippy::too_many_arguments)]
pub fn plan_hand_tick(
    current_height: f32,
    visible_texture: &Option<String>,
    visible_stack: &Option<ItemStack>,
    visible_custom_texture_key: &Option<String>,
    visible_using_item_context: bool,
    visible_pull_stage: u8,
    next_texture: &Option<String>,
    next_stack: &Option<ItemStack>,
    next_custom_texture_key: &Option<String>,
    next_using_item_context: bool,
    next_pull_stage: u8,
    raised_height: f32,
    hands_busy: bool,
    play_swap_animation: bool,
) -> HandTickPlan {
    let changed = hand_changed(
        visible_texture,
        visible_stack,
        visible_custom_texture_key,
        visible_using_item_context,
        visible_pull_stage,
        next_texture,
        next_stack,
        next_custom_texture_key,
        next_using_item_context,
        next_pull_stage,
    );
    let instant_replace = changed
        && should_instantly_replace_visible_item(visible_stack, next_stack, play_swap_animation);
    let target = if hands_busy {
        0.0
    } else if instant_replace {
        raised_height
    } else {
        hand_height_target(
            visible_texture,
            visible_stack,
            next_texture,
            next_stack,
            raised_height,
        )
    };
    let new_height = step_toward(current_height, target);
    let swap = should_swap_visible_item(new_height, changed);
    HandTickPlan {
        changed,
        instant_replace,
        target,
        new_height,
        swap,
    }
}
pub fn select_held_hands(
    main_stack: &ItemStack,
    off_stack: &ItemStack,
    using_hand_is_main: Option<bool>,
    is_spectator: bool,
) -> HeldHands {
    let selection = crate::render::item_in_hand_renderer::evaluate_which_hands_to_render_for_stacks(
        main_stack,
        off_stack,
        using_hand_is_main,
        is_spectator,
    );
    match (selection.render_main_hand, selection.render_off_hand) {
        (true, true) => HeldHands::Both,
        (true, false) => HeldHands::MainOnly,
        (false, true) => HeldHands::OffOnly,
        (false, false) => HeldHands::None,
    }
}
