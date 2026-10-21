#[allow(unused_imports)]
pub use crate::render::armor_pass::{AvatarOp, AvatarPipelineId, plan_avatar_ops};
#[allow(unused_imports)]
pub use crate::render::model_frame::{
    ArmorSlotFrameInput, CapeFrameInput, FrameLayerCommand, FrameLayerKind, HeldItemFrameInput,
    HumanoidFrameInput, ModelVisibility,
};
#[inline]
pub fn prepare_world_avatar_frame(input: &HumanoidFrameInput) -> Vec<FrameLayerCommand> {
    crate::render::model_frame::build_humanoid_frame(input)
}
#[inline]
pub fn plan_world_avatar_ops(frame: &[FrameLayerCommand]) -> Vec<AvatarOp> {
    plan_avatar_ops(frame)
}
#[inline]
pub fn run_world_avatar_frame(
    input: &HumanoidFrameInput,
) -> (Vec<FrameLayerCommand>, Vec<AvatarOp>) {
    let frame = prepare_world_avatar_frame(input);
    let ops = plan_world_avatar_ops(&frame);
    (frame, ops)
}
#[inline]
pub fn fallback_remote_visibility_current_behavior(
    invisible: bool,
    glowing: bool,
) -> ModelVisibility {
    ModelVisibility {
        invisible,
        invisible_to_player: invisible,
        glowing,
        is_spectator: false,
    }
}
