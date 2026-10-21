use crate::render::entity::living_entity_renderer::{LivingEntityRenderer, LivingEntityVisibility};
use el_core::item::ItemStack;
pub const WORLD_AVATAR_ARMOR_SLOTS: usize = 4;
pub const WORLD_ARMOR_DRAW_ORDER: [usize; 4] = [1, 2, 3, 0];
pub const WORLD_AVATAR_CROUCH_RENDER_OFFSET_Y: f32 = -2.0 / 16.0;
pub const WORLD_AVATAR_MODEL_SCALE: f32 = 0.9375;
pub const WORLD_CAPE_JACKET_OFFSET: [f32; 3] = [0.0, -0.053125, 0.06875];
pub const WORLD_LIVING_MODEL_ORIGIN_Y: f32 = 0.001;
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelVisibility {
    pub invisible: bool,
    pub invisible_to_player: bool,
    pub glowing: bool,
    pub is_spectator: bool,
}
impl ModelVisibility {
    pub const fn as_living(self) -> LivingEntityVisibility {
        LivingEntityVisibility {
            invisible: self.invisible,
            invisible_to_player: self.invisible_to_player,
            glowing: self.glowing,
        }
    }
}
#[inline]
pub fn should_submit_world_body(visibility: ModelVisibility) -> bool {
    LivingEntityRenderer::should_submit_body(visibility.as_living())
}
#[inline]
pub fn should_render_world_avatar_layers(is_spectator: bool) -> bool {
    !is_spectator
}
#[inline]
pub fn is_world_body_visible(visibility: ModelVisibility) -> bool {
    LivingEntityRenderer::is_body_visible(visibility.as_living())
}
#[inline]
pub fn fallback_hides_all_layers(
    invisible: bool,
    glowing: bool,
    is_player: bool,
    is_armor_stand: bool,
    armor_stand_equipped: bool,
) -> bool {
    invisible && !glowing && !is_player && !(is_armor_stand && armor_stand_equipped)
}
#[inline]
pub fn player_fallback_body_visible(invisible: bool, _glowing: bool) -> bool {
    !invisible
}
pub fn viewer_visibility(
    invisible: bool,
    viewer_spectator: bool,
    same_team: bool,
    see_friendly_invisibles: bool,
    glowing: bool,
    entity_spectator: bool,
) -> ModelVisibility {
    ModelVisibility {
        invisible,
        invisible_to_player: invisible
            && !viewer_spectator
            && !(same_team && see_friendly_invisibles),
        glowing,
        is_spectator: entity_spectator,
    }
}
pub fn world_body_color(visibility: ModelVisibility) -> [f32; 4] {
    [
        1.0,
        1.0,
        1.0,
        if LivingEntityRenderer::should_force_translucent(visibility.as_living()) {
            38.0 / 255.0
        } else {
            1.0
        },
    ]
}
pub fn should_submit_textured_world_body(visibility: ModelVisibility) -> bool {
    !visibility.invisible || !visibility.invisible_to_player
}
#[inline]
pub fn hurt_overlay_color(hurt_time: f32, death_time: f32) -> [f32; 4] {
    if hurt_time > 0.0 || death_time > 0.0 {
        [1.0, 0.0, 0.0, 178.0 / 255.0]
    } else {
        [1.0, 1.0, 1.0, 1.0]
    }
}
#[inline]
pub fn swim_body_pitch_degrees(swim_amount: f32, x_rot_degrees: f32, in_water: bool) -> f32 {
    let target = if in_water {
        -90.0 - x_rot_degrees
    } else {
        -90.0
    };
    swim_amount * target
}
#[inline]
pub fn avatar_crouch_offset_y(is_crouching: bool, scale: f32) -> f32 {
    if is_crouching {
        scale * WORLD_AVATAR_CROUCH_RENDER_OFFSET_Y
    } else {
        0.0
    }
}
#[inline]
pub fn camera_relative_position_d(
    entity_position: glam::DVec3,
    camera_position: glam::DVec3,
) -> glam::Vec3 {
    (entity_position - camera_position).as_vec3()
}
#[inline]
pub fn living_model_origin_y(is_living: bool) -> f32 {
    if is_living {
        WORLD_LIVING_MODEL_ORIGIN_Y
    } else {
        0.0
    }
}
pub fn world_avatar_body_matrix(
    camera_relative_position: glam::Vec3,
    render_offset: glam::Vec3,
    rotation: glam::Mat4,
    entity_scale: f32,
) -> glam::Mat4 {
    glam::Mat4::from_translation(camera_relative_position + render_offset)
        * glam::Mat4::from_scale(glam::Vec3::splat(entity_scale))
        * rotation
        * glam::Mat4::from_scale(glam::Vec3::splat(WORLD_AVATAR_MODEL_SCALE))
        * glam::Mat4::from_translation(glam::Vec3::Y * WORLD_LIVING_MODEL_ORIGIN_Y)
}
pub fn local_avatar_body_matrix(
    camera_relative_position: glam::Vec3,
    body_yaw_rad: f32,
    swim_amount: f32,
    x_rot_degrees: f32,
    in_water: bool,
    visually_swimming_shift: bool,
    entity_scale: f32,
    is_crouching: bool,
) -> glam::Mat4 {
    let rot = glam::Mat4::from_rotation_y(-body_yaw_rad);
    let rot = if swim_amount > 0.0 {
        let x_angle = swim_body_pitch_degrees(swim_amount, x_rot_degrees, in_water);
        let mut tilted = rot * glam::Mat4::from_rotation_x(-x_angle.to_radians());
        if visually_swimming_shift {
            tilted *= glam::Mat4::from_translation(glam::Vec3::new(0.0, -1.0, -0.3));
        }
        tilted
    } else {
        rot
    };
    world_avatar_body_matrix(
        camera_relative_position,
        glam::Vec3::Y * avatar_crouch_offset_y(is_crouching, entity_scale),
        rot,
        entity_scale,
    )
}
#[inline]
pub fn body_view_proj(view_proj: glam::Mat4, body_model: glam::Mat4) -> glam::Mat4 {
    view_proj * body_model
}
#[inline]
pub fn item_view_proj(
    view_proj: glam::Mat4,
    body_model: glam::Mat4,
    item_matrix: glam::Mat4,
) -> glam::Mat4 {
    view_proj * body_model * item_matrix
}
#[inline]
pub fn view_proj_cols(view_proj: glam::Mat4) -> [[f32; 4]; 4] {
    view_proj.to_cols_array_2d()
}
pub fn parse_armor_material(item_path: &str) -> Option<&'static str> {
    if item_path == "turtle_helmet" {
        return Some("turtle_scute");
    }
    let material = ["_helmet", "_chestplate", "_leggings", "_boots"]
        .iter()
        .find_map(|suffix| item_path.strip_suffix(suffix))?;
    Some(match material {
        "golden" => "gold",
        "leather" => "leather",
        "copper" => "copper",
        "chainmail" => "chainmail",
        "iron" => "iron",
        "diamond" => "diamond",
        "netherite" => "netherite",
        _ => return None,
    })
}
#[inline]
pub fn armor_layer_name(is_legs: bool) -> &'static str {
    if is_legs {
        "humanoid_leggings"
    } else {
        "humanoid"
    }
}
pub fn armor_texture_key_and_path(stack: &ItemStack, is_legs: bool) -> Option<(String, String)> {
    if stack.is_empty() {
        return None;
    }
    let material = parse_armor_material(stack.item.location.path.as_str())?;
    let layer = armor_layer_name(is_legs);
    Some((
        format!("armor:{layer}:{material}"),
        format!("entity/equipment/{layer}/{material}"),
    ))
}
#[inline]
pub fn should_draw_armor_slot(stack_empty: bool, texture_present: bool, mesh_empty: bool) -> bool {
    !stack_empty && texture_present && !mesh_empty
}
pub fn armor_rebuild_needed(
    cached_texture_key: Option<&str>,
    cached_stack: Option<&ItemStack>,
    cached_parts: Option<&crate::render::model::player::player_model::PlayerModelParts>,
    next_texture_key: &str,
    next_stack: &ItemStack,
    next_parts: &crate::render::model::player::player_model::PlayerModelParts,
) -> bool {
    let unchanged = cached_texture_key == Some(next_texture_key)
        && cached_stack.is_some_and(|current| current == next_stack)
        && cached_parts.is_some_and(|current| current == next_parts);
    !unchanged
}
#[inline]
pub fn chest_has_wings(chest_item_path: &str) -> bool {
    chest_item_path == "elytra"
}
#[inline]
pub fn should_draw_local_cape(show_cape: bool, texture_present: bool, has_wings: bool) -> bool {
    show_cape && texture_present && !has_wings
}
#[inline]
pub fn should_draw_remote_cape(
    invisible: bool,
    show_cape: bool,
    cape_present: bool,
    has_wings: bool,
) -> bool {
    !invisible && show_cape && cape_present && !has_wings
}
pub fn cape_rebuild_needed(
    cached_texture_key: Option<&str>,
    cached_parts: Option<&crate::render::model::player::player_model::PlayerModelParts>,
    next_texture_key: &str,
    next_parts: &crate::render::model::player::player_model::PlayerModelParts,
) -> bool {
    let unchanged =
        cached_texture_key == Some(next_texture_key) && cached_parts == Some(next_parts);
    !unchanged
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FrameLayerKind {
    Body,
    Armor(u8),
    MainItem,
    OffItem,
    Cape,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameLayerCommand {
    pub kind: FrameLayerKind,
    pub view_proj_cols: [[f32; 4]; 4],
    pub has_foil: bool,
    pub use_preview_pipeline: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmorSlotFrameInput {
    pub stack_empty: bool,
    pub texture_present: bool,
    pub mesh_empty: bool,
    pub has_foil: bool,
    pub view_proj_cols: [[f32; 4]; 4],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeldItemFrameInput {
    pub mesh_present: bool,
    pub index_count_nonzero: bool,
    pub has_foil: bool,
    pub custom_head_texture: bool,
    pub view_proj_cols: [[f32; 4]; 4],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapeFrameInput {
    pub show_cape: bool,
    pub texture_present: bool,
    pub has_wings: bool,
    pub invisible: bool,
    pub is_remote: bool,
    pub index_count_nonzero: bool,
    pub view_proj_cols: [[f32; 4]; 4],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HumanoidFrameInput {
    pub visibility: ModelVisibility,
    pub skin_available: bool,
    pub body_view_proj_cols: [[f32; 4]; 4],
    pub armor: [ArmorSlotFrameInput; WORLD_AVATAR_ARMOR_SLOTS],
    pub main_item: Option<HeldItemFrameInput>,
    pub off_item: Option<HeldItemFrameInput>,
    pub cape: CapeFrameInput,
}
fn cape_draws(input: &CapeFrameInput) -> bool {
    if !input.index_count_nonzero {
        return false;
    }
    if input.is_remote {
        should_draw_remote_cape(
            input.invisible,
            input.show_cape,
            input.texture_present,
            input.has_wings,
        )
    } else {
        should_draw_local_cape(input.show_cape, input.texture_present, input.has_wings)
    }
}
pub fn build_humanoid_frame(input: &HumanoidFrameInput) -> Vec<FrameLayerCommand> {
    let mut out = Vec::with_capacity(8);
    if should_submit_textured_world_body(input.visibility) && input.skin_available {
        out.push(FrameLayerCommand {
            kind: FrameLayerKind::Body,
            view_proj_cols: input.body_view_proj_cols,
            has_foil: false,
            use_preview_pipeline: false,
        });
    }
    if should_render_world_avatar_layers(input.visibility.is_spectator) {
        for index in WORLD_ARMOR_DRAW_ORDER {
            let slot = &input.armor[index];
            if should_draw_armor_slot(slot.stack_empty, slot.texture_present, slot.mesh_empty) {
                out.push(FrameLayerCommand {
                    kind: FrameLayerKind::Armor(index as u8),
                    view_proj_cols: slot.view_proj_cols,
                    has_foil: slot.has_foil,
                    use_preview_pipeline: false,
                });
            }
        }
        if let Some(item) = input.main_item {
            if item.mesh_present && item.index_count_nonzero {
                out.push(FrameLayerCommand {
                    kind: FrameLayerKind::MainItem,
                    view_proj_cols: item.view_proj_cols,
                    has_foil: item.has_foil,
                    use_preview_pipeline: item.custom_head_texture,
                });
            }
        }
        if let Some(item) = input.off_item {
            if item.mesh_present && item.index_count_nonzero {
                out.push(FrameLayerCommand {
                    kind: FrameLayerKind::OffItem,
                    view_proj_cols: item.view_proj_cols,
                    has_foil: item.has_foil,
                    use_preview_pipeline: item.custom_head_texture,
                });
            }
        }
        if cape_draws(&input.cape) {
            out.push(FrameLayerCommand {
                kind: FrameLayerKind::Cape,
                view_proj_cols: input.cape.view_proj_cols,
                has_foil: false,
                use_preview_pipeline: false,
            });
        }
    }
    out
}
#[inline]
pub fn frame_layer_order_key(kind: FrameLayerKind) -> u8 {
    match kind {
        FrameLayerKind::Body => 0,
        FrameLayerKind::Armor(slot) => match slot {
            1 => 10,
            2 => 11,
            3 => 12,
            _ => 13,
        },
        FrameLayerKind::MainItem => 20,
        FrameLayerKind::OffItem => 21,
        FrameLayerKind::Cape => 30,
    }
}
