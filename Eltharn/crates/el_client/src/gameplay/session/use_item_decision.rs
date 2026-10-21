#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseClickDecision {
    ReleaseActive,
    StartNew,
    HeldUse,
    None,
}
pub struct UseClickGuards {
    pub has_click: bool,
    pub is_using_item: bool,
    pub right_click_delay: i32,
    pub is_use_pressed: bool,
    pub hands_busy: bool,
    pub has_target: bool,
}
pub fn classify_use_tick(guards: &UseClickGuards) -> UseClickDecision {
    if guards.is_using_item && !guards.is_use_pressed {
        return UseClickDecision::ReleaseActive;
    }
    if guards.has_click && !guards.is_using_item {
        return UseClickDecision::StartNew;
    }
    if !guards.is_using_item && guards.is_use_pressed && guards.right_click_delay == 0 {
        return UseClickDecision::HeldUse;
    }
    if guards.is_using_item {
        return UseClickDecision::HeldUse;
    }
    UseClickDecision::None
}
