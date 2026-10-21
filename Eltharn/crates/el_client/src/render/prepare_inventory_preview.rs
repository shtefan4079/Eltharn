use crate::render::baked_model::Vertex;
use crate::render::model::player::player_model::{
    ArmorSlot, HandSide, PlayerModelParts, third_person_item_in_hand_matrix,
};
use el_core::item::ItemStack;
pub const PREVIEW_RENDER_STATE_PARTIAL_TICK: f32 = 1.0;
pub const PREVIEW_CROUCH_RENDER_OFFSET_Y: f32 = -2.0 / 16.0;
pub const PREVIEW_LIVING_ENTITY_MODEL_ORIGIN_Y: f32 = 1.501 - 1.5;
pub const PREVIEW_MODEL_BASIS_YAW: f32 = std::f32::consts::PI;
pub const PREVIEW_REVERSE_DEPTH_CLEAR_VALUE: f32 = 0.0;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewAngles {
    pub x_angle: f32,
    pub y_angle: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewScissor {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
#[inline]
pub fn preview_center(x0: f32, x1: f32, y0: f32, y1: f32) -> (f32, f32) {
    ((x0 + x1) * 0.5, (y0 + y1) * 0.5)
}
#[inline]
pub fn preview_angles(center_x: f32, center_y: f32, mouse_x: f32, mouse_y: f32) -> PreviewAngles {
    PreviewAngles {
        x_angle: ((center_x - mouse_x) / 40.0).atan(),
        y_angle: ((center_y - mouse_y) / 40.0).atan(),
    }
}
#[inline]
pub fn preview_logical_size(screen_w: f32, screen_h: f32, gui_scale: f32) -> (f32, f32) {
    (screen_w / gui_scale, screen_h / gui_scale)
}
pub fn preview_scissor(
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
    gui_scale: f32,
    config_w: u32,
    config_h: u32,
) -> Option<PreviewScissor> {
    let scissor_x0 = (x0 * gui_scale).floor().clamp(0.0, config_w as f32) as u32;
    let scissor_y0 = (y0 * gui_scale).floor().clamp(0.0, config_h as f32) as u32;
    let scissor_x1 = (x1 * gui_scale).ceil().clamp(0.0, config_w as f32) as u32;
    let scissor_y1 = (y1 * gui_scale).ceil().clamp(0.0, config_h as f32) as u32;
    let width = scissor_x1.saturating_sub(scissor_x0);
    let height = scissor_y1.saturating_sub(scissor_y0);
    if width == 0 || height == 0 {
        return None;
    }
    Some(PreviewScissor {
        x: scissor_x0,
        y: scissor_y0,
        width,
        height,
    })
}
#[inline]
pub fn preview_projection(logical_w: f32, logical_h: f32) -> glam::Mat4 {
    glam::Mat4::orthographic_rh(0.0, logical_w, logical_h, 0.0, 1000.0, -1000.0)
}
#[inline]
pub fn preview_avatar_offset_y(is_crouching: bool) -> f32 {
    if is_crouching {
        PREVIEW_CROUCH_RENDER_OFFSET_Y
    } else {
        0.0
    }
}
#[inline]
pub fn preview_entity_translation_y(
    bounding_box_height: f32,
    offset_y: f32,
    crouch_offset_y: f32,
) -> f32 {
    bounding_box_height * 0.5 + offset_y + crouch_offset_y
}
#[allow(clippy::too_many_arguments)]
pub fn preview_model_matrix(
    center_x: f32,
    center_y: f32,
    size: f32,
    entity_translation_y: f32,
    _x_angle: f32,
    y_angle: f32,
    mouse_yaw: f32,
) -> glam::Mat4 {
    let pip_translation = glam::Mat4::from_translation(glam::Vec3::new(center_x, center_y, 0.0));
    let pip_scale = glam::Mat4::from_scale(glam::Vec3::new(size, size, -size));
    let entity_translation =
        glam::Mat4::from_translation(glam::Vec3::new(0.0, entity_translation_y, 0.0));
    let gui_rotation = glam::Mat4::from_rotation_z(std::f32::consts::PI)
        * glam::Mat4::from_rotation_x(y_angle * 20.0_f32.to_radians());
    let body_rotation = glam::Mat4::from_rotation_y(-mouse_yaw);
    let avatar_scale = glam::Mat4::from_scale(glam::Vec3::splat(0.9375));
    let model_basis =
        glam::Mat4::from_translation(glam::Vec3::Y * PREVIEW_LIVING_ENTITY_MODEL_ORIGIN_Y)
            * glam::Mat4::from_rotation_y(PREVIEW_MODEL_BASIS_YAW);
    pip_translation
        * pip_scale
        * entity_translation
        * gui_rotation
        * body_rotation
        * avatar_scale
        * model_basis
}
#[inline]
pub fn preview_player_view_proj(proj: glam::Mat4, model: glam::Mat4) -> glam::Mat4 {
    proj * model
}
#[inline]
pub fn preview_item_view_proj(
    player_view_proj: glam::Mat4,
    parts: &PlayerModelParts,
    hand: HandSide,
) -> glam::Mat4 {
    player_view_proj * third_person_item_in_hand_matrix(parts, hand)
}
#[inline]
pub fn preview_item_view_proj_cols(
    player_view_proj: glam::Mat4,
    parts: &PlayerModelParts,
    hand: HandSide,
) -> [[f32; 4]; 4] {
    preview_item_view_proj(player_view_proj, parts, hand).to_cols_array_2d()
}
#[inline]
pub fn build_preview_body_mesh(parts: &PlayerModelParts) -> (Vec<Vertex>, Vec<u32>) {
    crate::render::entity::player::avatar_renderer::AvatarRenderer::generate_player_mesh(parts)
}
#[inline]
pub fn build_preview_armor_mesh(
    parts: &PlayerModelParts,
    slot: ArmorSlot,
) -> (Vec<Vertex>, Vec<u32>) {
    crate::render::model::player::player_model::generate_armor_mesh(parts, slot)
}
#[inline]
pub fn build_preview_cape_mesh(
    parts: &PlayerModelParts,
    cape_flap: f32,
    cape_lean: f32,
    cape_lean2: f32,
    has_humanoid: bool,
) -> (Vec<Vertex>, Vec<u32>) {
    crate::render::model::player::player_model::generate_cape_mesh_with_jacket(
        parts,
        cape_flap,
        cape_lean,
        cape_lean2,
        has_humanoid,
    )
}
#[inline]
pub fn preview_armor_draws(stack_empty: bool, texture_present: bool) -> bool {
    !stack_empty && texture_present
}
#[inline]
pub fn preview_armor_mesh_is_empty(vertices_empty: bool, indices_empty: bool) -> bool {
    vertices_empty || indices_empty
}
pub fn preview_armor_rebuild_needed(
    cached_texture_key: Option<&str>,
    cached_stack: Option<&ItemStack>,
    cached_parts: Option<&PlayerModelParts>,
    next_texture_key: &str,
    next_stack: &ItemStack,
    next_parts: &PlayerModelParts,
) -> bool {
    let unchanged = cached_texture_key == Some(next_texture_key)
        && cached_stack.is_some_and(|current| current == next_stack)
        && cached_parts.is_some_and(|current| current == next_parts);
    !unchanged
}
#[inline]
pub fn preview_cape_draws(show_cape: bool, texture_present: bool, has_wings: bool) -> bool {
    show_cape && texture_present && !has_wings
}
pub fn preview_cape_rebuild_needed(
    cached_texture_key: Option<&str>,
    cached_parts: Option<&PlayerModelParts>,
    next_texture_key: &str,
    next_parts: &PlayerModelParts,
) -> bool {
    let unchanged =
        cached_texture_key == Some(next_texture_key) && cached_parts == Some(next_parts);
    !unchanged
}
