#![allow(dead_code)]
use crate::render::baked_model::Vertex;
use glam::{Mat4, Quat, Vec3};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandSide {
    Right,
    Left,
}
impl HandSide {
    fn invert(self) -> f32 {
        match self {
            Self::Right => 1.0,
            Self::Left => -1.0,
        }
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerModelParts {
    pub head_yaw: f32,
    pub head_pitch: f32,
    pub head_y_offset: f32,
    pub body_yaw: f32,
    pub body_pitch: f32,
    pub body_y_offset: f32,
    pub right_arm_pitch: f32,
    pub right_arm_yaw: f32,
    pub right_arm_roll: f32,
    pub right_arm_x_offset: f32,
    pub right_arm_y_offset: f32,
    pub right_arm_z_offset: f32,
    pub left_arm_pitch: f32,
    pub left_arm_yaw: f32,
    pub left_arm_roll: f32,
    pub left_arm_x_offset: f32,
    pub left_arm_y_offset: f32,
    pub left_arm_z_offset: f32,
    pub right_leg_pitch: f32,
    pub right_leg_yaw: f32,
    pub right_leg_roll: f32,
    pub right_leg_z_offset: f32,
    pub left_leg_pitch: f32,
    pub left_leg_yaw: f32,
    pub left_leg_roll: f32,
    pub left_leg_z_offset: f32,
    pub slim: bool,
    pub show_body: bool,
    pub show_hat: bool,
    pub show_jacket: bool,
    pub show_left_pants: bool,
    pub show_right_pants: bool,
    pub show_left_sleeve: bool,
    pub show_right_sleeve: bool,
    pub show_cape: bool,
    pub cape_flap: f32,
    pub cape_lean: f32,
    pub cape_lean2: f32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerModel {
    pub slim: bool,
}
#[allow(dead_code)]
impl PlayerModel {
    pub const fn new(slim: bool) -> Self {
        Self { slim }
    }
    pub fn setup_anim(self, player: &crate::gameplay::player::LocalPlayer) -> PlayerModelParts {
        let mut parts = setup_anim(player);
        parts.slim = self.slim;
        parts
    }
    pub fn mesh(self, parts: &PlayerModelParts) -> (Vec<Vertex>, Vec<u32>) {
        let mut parts = parts.clone();
        parts.slim = self.slim;
        generate_player_mesh(&parts)
    }
    pub fn right_hand_mesh(self, show_sleeve: bool) -> (Vec<Vertex>, Vec<u32>) {
        generate_first_person_arm(self.slim, HandSide::Right, show_sleeve)
    }
    pub fn left_hand_mesh(self, show_sleeve: bool) -> (Vec<Vertex>, Vec<u32>) {
        generate_first_person_arm(self.slim, HandSide::Left, show_sleeve)
    }
}
impl Default for PlayerModelParts {
    fn default() -> Self {
        Self {
            head_yaw: 0.0,
            head_pitch: 0.0,
            head_y_offset: 0.0,
            body_yaw: 0.0,
            body_pitch: 0.0,
            body_y_offset: 0.0,
            right_arm_pitch: 0.0,
            right_arm_yaw: 0.0,
            right_arm_roll: 0.0,
            right_arm_x_offset: 0.0,
            right_arm_y_offset: 0.0,
            right_arm_z_offset: 0.0,
            left_arm_pitch: 0.0,
            left_arm_yaw: 0.0,
            left_arm_roll: 0.0,
            left_arm_x_offset: 0.0,
            left_arm_y_offset: 0.0,
            left_arm_z_offset: 0.0,
            right_leg_pitch: 0.0,
            right_leg_yaw: 0.0,
            right_leg_roll: 0.0,
            right_leg_z_offset: 0.0,
            left_leg_pitch: 0.0,
            left_leg_yaw: 0.0,
            left_leg_roll: 0.0,
            left_leg_z_offset: 0.0,
            slim: false,
            show_body: true,
            show_hat: true,
            show_jacket: true,
            show_left_pants: true,
            show_right_pants: true,
            show_left_sleeve: true,
            show_right_sleeve: true,
            show_cape: false,
            cape_flap: 0.0,
            cape_lean: 0.0,
            cape_lean2: 0.0,
        }
    }
}
pub fn setup_anim(player: &crate::gameplay::player::LocalPlayer) -> PlayerModelParts {
    setup_anim_with_walk(
        player,
        player.walk_animation_pos,
        player.walk_animation_speed,
    )
}
pub fn setup_anim_with_walk(
    player: &crate::gameplay::player::LocalPlayer,
    animation_pos: f32,
    animation_speed: f32,
) -> PlayerModelParts {
    setup_anim_with_walk_and_rot(
        player,
        animation_pos,
        animation_speed,
        player.body_yaw,
        player.head_yaw,
    )
}
pub fn setup_anim_with_walk_and_rot(
    player: &crate::gameplay::player::LocalPlayer,
    animation_pos: f32,
    animation_speed: f32,
    body_yaw: f32,
    head_yaw: f32,
) -> PlayerModelParts {
    let mut state = crate::render::entity::state::HumanoidRenderState::from_player(player);
    state.walk_animation_pos = animation_pos;
    state.walk_animation_speed = animation_speed;
    state.y_rot = wrap_radians(head_yaw - body_yaw).to_degrees();
    crate::render::model::humanoid_model::HumanoidModel::setup_anim_state(&state)
}
pub fn apply_item_arm_pose(parts: &mut PlayerModelParts, hand: HandSide) {
    match hand {
        HandSide::Right => {
            parts.right_arm_pitch = parts.right_arm_pitch * 0.5 - std::f32::consts::PI / 10.0;
            parts.right_arm_yaw = 0.0;
        }
        HandSide::Left => {
            parts.left_arm_pitch = parts.left_arm_pitch * 0.5 - std::f32::consts::PI / 10.0;
            parts.left_arm_yaw = 0.0;
        }
    }
}
fn wrap_radians(value: f32) -> f32 {
    let two_pi = std::f32::consts::TAU;
    (value + std::f32::consts::PI).rem_euclid(two_pi) - std::f32::consts::PI
}
pub(super) fn java_model_part_rotation(pitch: f32, yaw: f32, roll: f32) -> Quat {
    Quat::from_euler(glam::EulerRot::ZYX, -roll, -yaw, pitch)
}
pub fn apply_inventory_preview_pose(
    parts: &mut PlayerModelParts,
    x_angle: f32,
    y_angle: f32,
    is_fall_flying: bool,
) -> f32 {
    let mouse_yaw = x_angle * 20.0_f32.to_radians();
    parts.head_yaw = mouse_yaw;
    if !is_fall_flying {
        parts.head_pitch = -y_angle * 20.0_f32.to_radians();
    }
    mouse_yaw
}
pub fn setup_inventory_anim(
    player: &crate::gameplay::player::LocalPlayer,
    x_angle: f32,
    y_angle: f32,
) -> PlayerModelParts {
    let mut parts = setup_anim(player);
    apply_inventory_preview_pose(&mut parts, x_angle, y_angle, player.is_fall_flying);
    parts
}
pub fn vanilla_view_bob_matrix(walk_animation_pos: f32, bob: f32) -> Mat4 {
    let walk_dist_anim = -walk_animation_pos;
    let bob_x = (walk_dist_anim * std::f32::consts::PI).sin() * bob * 0.5;
    let bob_y = -((walk_dist_anim * std::f32::consts::PI).cos() * bob).abs();
    let bob_rot_z = (walk_dist_anim * std::f32::consts::PI).sin() * bob * 3.0;
    let bob_rot_x = ((walk_dist_anim * std::f32::consts::PI - 0.2).cos() * bob).abs() * 5.0;
    Mat4::from_translation(Vec3::new(bob_x, bob_y, 0.0))
        * Mat4::from_rotation_z(bob_rot_z.to_radians())
        * Mat4::from_rotation_x(bob_rot_x.to_radians())
}
pub fn vanilla_hand_camera_bob_matrix(
    view_pitch: f32,
    view_yaw: f32,
    x_bob: f32,
    y_bob: f32,
) -> Mat4 {
    Mat4::from_rotation_x((view_pitch - x_bob) * 0.1)
        * Mat4::from_rotation_y(wrap_radians(view_yaw - y_bob) * 0.1)
}
pub fn third_person_item_in_hand_matrix(parts: &PlayerModelParts, hand: HandSide) -> Mat4 {
    let px = 1.0 / 16.0;
    let invert = hand.invert();
    let slim_hand_offset = if parts.slim { 0.5 * invert } else { 0.0 };
    let y_dir = -1.0;
    let z_dir = -1.0;
    let (pivot, rot) = match hand {
        HandSide::Right => (
            Vec3::new(
                -5.0 + slim_hand_offset + parts.right_arm_x_offset,
                22.0 + parts.right_arm_y_offset * y_dir,
                parts.right_arm_z_offset * z_dir,
            ) * px,
            java_model_part_rotation(
                parts.right_arm_pitch,
                parts.right_arm_yaw,
                parts.right_arm_roll,
            ),
        ),
        HandSide::Left => (
            Vec3::new(
                5.0 + slim_hand_offset + parts.left_arm_x_offset,
                22.0 + parts.left_arm_y_offset * y_dir,
                parts.left_arm_z_offset * z_dir,
            ) * px,
            java_model_part_rotation(
                parts.left_arm_pitch,
                parts.left_arm_yaw,
                parts.left_arm_roll,
            ),
        ),
    };
    Mat4::from_translation(pivot) * Mat4::from_quat(rot) * third_person_item_layer_matrix(hand)
}
pub fn third_person_item_layer_matrix(hand: HandSide) -> Mat4 {
    let vanilla_offset = Vec3::new(hand.invert() / 16.0, 2.0 / 16.0, -10.0 / 16.0);
    let java_item_in_hand_layer = Mat4::from_rotation_x(-90.0_f32.to_radians())
        * Mat4::from_rotation_y(180.0_f32.to_radians())
        * Mat4::from_translation(vanilla_offset);
    Mat4::from_rotation_x(180.0_f32.to_radians()) * java_item_in_hand_layer
}
pub fn vanilla_render_player_arm_matrix(
    hand: HandSide,
    inverse_arm_height: f32,
    attack_value: f32,
) -> Mat4 {
    let invert = hand.invert();
    let sqrt_attack_value = attack_value.sqrt();
    let x_swing_position = -0.3 * (sqrt_attack_value * std::f32::consts::PI).sin();
    let y_swing_position = 0.4 * (sqrt_attack_value * std::f32::consts::PI * 2.0).sin();
    let z_swing_position = -0.4 * (attack_value * std::f32::consts::PI).sin();
    let z_swing_rotation = (attack_value * attack_value * std::f32::consts::PI).sin();
    let y_swing_rotation = (sqrt_attack_value * std::f32::consts::PI).sin();
    Mat4::from_translation(Vec3::new(
        invert * (x_swing_position + 0.64000005),
        y_swing_position - 0.6 + inverse_arm_height * -0.6,
        z_swing_position - 0.71999997,
    )) * Mat4::from_rotation_y((invert * 45.0).to_radians())
        * Mat4::from_rotation_y((invert * y_swing_rotation * 70.0).to_radians())
        * Mat4::from_rotation_z((invert * z_swing_rotation * -20.0).to_radians())
        * Mat4::from_translation(Vec3::new(invert * -1.0, 3.6, 3.5))
        * Mat4::from_rotation_z((invert * 120.0).to_radians())
        * Mat4::from_rotation_x(200.0_f32.to_radians())
        * Mat4::from_rotation_y((invert * -135.0).to_radians())
        * Mat4::from_translation(Vec3::new(invert * 5.6, 0.0, 0.0))
}
pub fn vanilla_item_in_hand_matrix(
    hand: HandSide,
    inverse_arm_height: f32,
    attack_value: f32,
) -> Mat4 {
    let invert = hand.invert();
    let sqrt_attack = attack_value.sqrt();
    let x_swing_position = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
    let y_swing_position = 0.2 * (sqrt_attack * std::f32::consts::PI * 2.0).sin();
    let z_swing_position = -0.2 * (attack_value * std::f32::consts::PI).sin();
    let y_swing_rotation = (attack_value * attack_value * std::f32::consts::PI).sin();
    let xz_swing_rotation = (sqrt_attack * std::f32::consts::PI).sin();
    Mat4::from_translation(Vec3::new(
        invert * 0.56,
        -0.52 + inverse_arm_height * -0.6,
        -0.72,
    )) * Mat4::from_translation(Vec3::new(
        invert * x_swing_position,
        y_swing_position,
        z_swing_position,
    )) * Mat4::from_rotation_y((invert * (45.0 + y_swing_rotation * -20.0)).to_radians())
        * Mat4::from_rotation_z((invert * xz_swing_rotation * -20.0).to_radians())
        * Mat4::from_rotation_x((xz_swing_rotation * -80.0).to_radians())
        * Mat4::from_rotation_y((invert * -45.0).to_radians())
}
pub fn generate_player_mesh(parts: &PlayerModelParts) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let body_rot = java_model_part_rotation(parts.body_pitch, parts.body_yaw, 0.0);
    let head_rot = java_model_part_rotation(parts.head_pitch, parts.head_yaw, 0.0);
    let right_arm_rot = java_model_part_rotation(
        parts.right_arm_pitch,
        parts.right_arm_yaw,
        parts.right_arm_roll,
    );
    let left_arm_rot = java_model_part_rotation(
        parts.left_arm_pitch,
        parts.left_arm_yaw,
        parts.left_arm_roll,
    );
    let right_leg_rot = java_model_part_rotation(
        parts.right_leg_pitch,
        parts.right_leg_yaw,
        parts.right_leg_roll,
    );
    let left_leg_rot = java_model_part_rotation(
        parts.left_leg_pitch,
        parts.left_leg_yaw,
        parts.left_leg_roll,
    );
    let px = 1.0 / 16.0;
    let tex_w = 64.0;
    let tex_h = 64.0;
    let mut add_cuboid =
        |origin: Vec3, size: Vec3, rot: Quat, pivot: Vec3, uv: [f32; 2], uv_size: [f32; 3]| {
            let (u, v) = (uv[0], uv[1]);
            let (dx, dy, dz) = (uv_size[0], uv_size[1], uv_size[2]);
            let sx = size.x * px;
            let sy = size.y * px;
            let sz = size.z * px;
            let ox = origin.x * px;
            let oy = origin.y * px;
            let oz = origin.z * px;
            let px_pivot = pivot * px;
            add_quad_3d(
                &mut vertices,
                &mut indices,
                Vec3::new(ox, oy + sy, oz + sz),
                Vec3::new(ox, oy, oz + sz),
                Vec3::new(ox + sx, oy, oz + sz),
                Vec3::new(ox + sx, oy + sy, oz + sz),
                rot,
                px_pivot,
                [u + dz, v + dz],
                [dx, dy],
                tex_w,
                tex_h,
                Vec3::Z,
            );
            add_quad_3d(
                &mut vertices,
                &mut indices,
                Vec3::new(ox + sx, oy + sy, oz),
                Vec3::new(ox + sx, oy, oz),
                Vec3::new(ox, oy, oz),
                Vec3::new(ox, oy + sy, oz),
                rot,
                px_pivot,
                [u + dz + dx + dz, v + dz],
                [dx, dy],
                tex_w,
                tex_h,
                -Vec3::Z,
            );
            add_quad_3d(
                &mut vertices,
                &mut indices,
                Vec3::new(ox, oy + sy, oz),
                Vec3::new(ox, oy + sy, oz + sz),
                Vec3::new(ox + sx, oy + sy, oz + sz),
                Vec3::new(ox + sx, oy + sy, oz),
                rot,
                px_pivot,
                [u + dz, v],
                [dx, dz],
                tex_w,
                tex_h,
                Vec3::Y,
            );
            add_quad_3d(
                &mut vertices,
                &mut indices,
                Vec3::new(ox, oy, oz + sz),
                Vec3::new(ox, oy, oz),
                Vec3::new(ox + sx, oy, oz),
                Vec3::new(ox + sx, oy, oz + sz),
                rot,
                px_pivot,
                [u + dz + dx, v],
                [dx, dz],
                tex_w,
                tex_h,
                -Vec3::Y,
            );
            add_quad_3d(
                &mut vertices,
                &mut indices,
                Vec3::new(ox, oy + sy, oz),
                Vec3::new(ox, oy, oz),
                Vec3::new(ox, oy, oz + sz),
                Vec3::new(ox, oy + sy, oz + sz),
                rot,
                px_pivot,
                [u, v + dz],
                [dz, dy],
                tex_w,
                tex_h,
                -Vec3::X,
            );
            add_quad_3d(
                &mut vertices,
                &mut indices,
                Vec3::new(ox + sx, oy + sy, oz + sz),
                Vec3::new(ox + sx, oy, oz + sz),
                Vec3::new(ox + sx, oy, oz),
                Vec3::new(ox + sx, oy + sy, oz),
                rot,
                px_pivot,
                [u + dz + dx, v + dz],
                [dz, dy],
                tex_w,
                tex_h,
                Vec3::X,
            );
        };
    let y_dir = -1.0;
    let z_dir = -1.0;
    let body_pivot = Vec3::new(0.0, 24.0 + parts.body_y_offset * y_dir, 0.0);
    if parts.show_body {
        add_cuboid(
            Vec3::new(-4.0, 12.0 + parts.body_y_offset * y_dir, -2.0),
            Vec3::new(8.0, 12.0, 4.0),
            body_rot,
            body_pivot,
            [16.0, 16.0],
            [8.0, 12.0, 4.0],
        );
        if parts.show_jacket {
            let overlay = 0.25;
            add_cuboid(
                Vec3::new(
                    -4.0 - overlay,
                    12.0 + parts.body_y_offset * y_dir - overlay,
                    -2.0 - overlay,
                ),
                Vec3::new(
                    8.0 + overlay * 2.0,
                    12.0 + overlay * 2.0,
                    4.0 + overlay * 2.0,
                ),
                body_rot,
                body_pivot,
                [16.0, 32.0],
                [8.0, 12.0, 4.0],
            );
        }
    }
    add_cuboid(
        Vec3::new(-4.0, 24.0 + parts.head_y_offset * y_dir, -4.0),
        Vec3::new(8.0, 8.0, 8.0),
        head_rot,
        Vec3::new(0.0, 24.0 + parts.head_y_offset * y_dir, 0.0),
        [0.0, 0.0],
        [8.0, 8.0, 8.0],
    );
    if parts.show_hat {
        let overlay = 0.5;
        add_cuboid(
            Vec3::new(
                -4.0 - overlay,
                24.0 + parts.head_y_offset * y_dir - overlay,
                -4.0 - overlay,
            ),
            Vec3::splat(8.0 + overlay * 2.0),
            head_rot,
            Vec3::new(0.0, 24.0 + parts.head_y_offset * y_dir, 0.0),
            [32.0, 0.0],
            [8.0, 8.0, 8.0],
        );
    }
    let arm_w = if parts.slim { 3.0 } else { 4.0 };
    let right_arm_pivot = Vec3::new(
        -5.0 + parts.right_arm_x_offset,
        22.0 + parts.right_arm_y_offset * y_dir,
        parts.right_arm_z_offset * z_dir,
    );
    if parts.show_body {
        add_cuboid(
            Vec3::new(
                -4.0 - arm_w + parts.right_arm_x_offset,
                12.0 + parts.right_arm_y_offset * y_dir,
                -2.0 + parts.right_arm_z_offset * z_dir,
            ),
            Vec3::new(arm_w, 12.0, 4.0),
            right_arm_rot,
            right_arm_pivot,
            [40.0, 16.0],
            [arm_w, 12.0, 4.0],
        );
        if parts.show_right_sleeve {
            let overlay = 0.25;
            add_cuboid(
                Vec3::new(
                    -4.0 - arm_w + parts.right_arm_x_offset - overlay,
                    12.0 + parts.right_arm_y_offset * y_dir - overlay,
                    -2.0 + parts.right_arm_z_offset * z_dir - overlay,
                ),
                Vec3::new(
                    arm_w + overlay * 2.0,
                    12.0 + overlay * 2.0,
                    4.0 + overlay * 2.0,
                ),
                right_arm_rot,
                right_arm_pivot,
                [40.0, 32.0],
                [arm_w, 12.0, 4.0],
            );
        }
    }
    let left_arm_pivot = Vec3::new(
        5.0 + parts.left_arm_x_offset,
        22.0 + parts.left_arm_y_offset * y_dir,
        parts.left_arm_z_offset * z_dir,
    );
    if parts.show_body {
        add_cuboid(
            Vec3::new(
                4.0 + parts.left_arm_x_offset,
                12.0 + parts.left_arm_y_offset * y_dir,
                -2.0 + parts.left_arm_z_offset * z_dir,
            ),
            Vec3::new(arm_w, 12.0, 4.0),
            left_arm_rot,
            left_arm_pivot,
            [32.0, 48.0],
            [arm_w, 12.0, 4.0],
        );
        if parts.show_left_sleeve {
            let overlay = 0.25;
            add_cuboid(
                Vec3::new(
                    4.0 + parts.left_arm_x_offset - overlay,
                    12.0 + parts.left_arm_y_offset * y_dir - overlay,
                    -2.0 + parts.left_arm_z_offset * z_dir - overlay,
                ),
                Vec3::new(
                    arm_w + overlay * 2.0,
                    12.0 + overlay * 2.0,
                    4.0 + overlay * 2.0,
                ),
                left_arm_rot,
                left_arm_pivot,
                [48.0, 48.0],
                [arm_w, 12.0, 4.0],
            );
        }
    }
    let right_leg_pivot = Vec3::new(-1.9, 12.0, parts.right_leg_z_offset * z_dir);
    if parts.show_body {
        add_cuboid(
            Vec3::new(-3.9, 0.0, -2.0 + parts.right_leg_z_offset * z_dir),
            Vec3::new(4.0, 12.0, 4.0),
            right_leg_rot,
            right_leg_pivot,
            [0.0, 16.0],
            [4.0, 12.0, 4.0],
        );
        if parts.show_right_pants {
            let overlay = 0.25;
            add_cuboid(
                Vec3::new(
                    -3.9 - overlay,
                    -overlay,
                    -2.0 + parts.right_leg_z_offset * z_dir - overlay,
                ),
                Vec3::new(
                    4.0 + overlay * 2.0,
                    12.0 + overlay * 2.0,
                    4.0 + overlay * 2.0,
                ),
                right_leg_rot,
                right_leg_pivot,
                [0.0, 32.0],
                [4.0, 12.0, 4.0],
            );
        }
    }
    let left_leg_pivot = Vec3::new(1.9, 12.0, parts.left_leg_z_offset * z_dir);
    if parts.show_body {
        add_cuboid(
            Vec3::new(-0.1, 0.0, -2.0 + parts.left_leg_z_offset * z_dir),
            Vec3::new(4.0, 12.0, 4.0),
            left_leg_rot,
            left_leg_pivot,
            [16.0, 48.0],
            [4.0, 12.0, 4.0],
        );
        if parts.show_left_pants {
            let overlay = 0.25;
            add_cuboid(
                Vec3::new(
                    -0.1 - overlay,
                    -overlay,
                    -2.0 + parts.left_leg_z_offset * z_dir - overlay,
                ),
                Vec3::new(
                    4.0 + overlay * 2.0,
                    12.0 + overlay * 2.0,
                    4.0 + overlay * 2.0,
                ),
                left_leg_rot,
                left_leg_pivot,
                [0.0, 48.0],
                [4.0, 12.0, 4.0],
            );
        }
    }
    (vertices, indices)
}
pub fn generate_cape_mesh(
    parts: &PlayerModelParts,
    cape_flap: f32,
    cape_lean: f32,
    cape_lean2: f32,
) -> (Vec<Vertex>, Vec<u32>) {
    generate_cape_mesh_with_jacket(parts, cape_flap, cape_lean, cape_lean2, false)
}
pub fn generate_cape_mesh_with_jacket(
    parts: &PlayerModelParts,
    cape_flap: f32,
    cape_lean: f32,
    cape_lean2: f32,
    jacket_offset: bool,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let body_rot = java_model_part_rotation(parts.body_pitch, parts.body_yaw, 0.0);
    let y_dir = -1.0;
    let body_pivot = Vec3::new(0.0, 24.0 + parts.body_y_offset * y_dir, 0.0);
    let cape_offset = Vec3::new(0.0, 0.0, -2.0);
    let px = 1.0 / 16.0;
    let jacket_offset = if jacket_offset {
        Vec3::new(0.0, -0.053125 * 16.0, -0.06875 * 16.0)
    } else {
        Vec3::ZERO
    };
    let alpha_x = (6.0 + cape_lean / 2.0 + cape_flap).to_radians();
    let alpha_z = (cape_lean2 / 2.0).to_radians();
    let theta = (180.0 - cape_lean2 / 2.0).to_radians();
    let cape_rot = Quat::from_rotation_x(alpha_x)
        * Quat::from_rotation_z(-alpha_z)
        * Quat::from_rotation_y(-theta);
    let (t0, t1, t2, t3) = (
        Vec3::new(-5.0, 24.0, 1.0),
        Vec3::new(5.0, 24.0, 1.0),
        Vec3::new(5.0, 8.0, 1.0),
        Vec3::new(-5.0, 8.0, 1.0),
    );
    let (l0, l1, l2, l3) = (
        Vec3::new(-5.0, 24.0, 0.0),
        Vec3::new(5.0, 24.0, 0.0),
        Vec3::new(5.0, 8.0, 0.0),
        Vec3::new(-5.0, 8.0, 0.0),
    );
    let tex_w = 64.0;
    let tex_h = 32.0;
    let faces: [(Vec3, Vec3, Vec3, Vec3, [[f32; 2]; 4], Vec3); 6] = [
        (
            l1,
            l0,
            t0,
            t1,
            [[11.0, 0.0], [1.0, 0.0], [1.0, 1.0], [11.0, 1.0]],
            Vec3::Y,
        ),
        (
            t2,
            t3,
            l3,
            l2,
            [[21.0, 1.0], [11.0, 1.0], [11.0, 0.0], [21.0, 0.0]],
            Vec3::NEG_Y,
        ),
        (
            t0,
            l0,
            l3,
            t3,
            [[1.0, 1.0], [0.0, 1.0], [0.0, 17.0], [1.0, 17.0]],
            Vec3::NEG_X,
        ),
        (
            t1,
            t0,
            t3,
            t2,
            [[11.0, 1.0], [1.0, 1.0], [1.0, 17.0], [11.0, 17.0]],
            Vec3::Z,
        ),
        (
            l1,
            t1,
            t2,
            l2,
            [[12.0, 1.0], [11.0, 1.0], [11.0, 17.0], [12.0, 17.0]],
            Vec3::X,
        ),
        (
            l0,
            l1,
            l2,
            l3,
            [[22.0, 1.0], [12.0, 1.0], [12.0, 17.0], [22.0, 17.0]],
            Vec3::NEG_Z,
        ),
    ];
    for (a, b, c, d, uvs, java_normal) in faces {
        let n = body_rot * (cape_rot * java_normal);
        let n = n.normalize();
        let transform = |corner: Vec3| -> Vec3 {
            let p = cape_offset + cape_rot * (corner - body_pivot);
            let p = body_pivot + body_rot * p;
            (p + jacket_offset) * px
        };
        let pa = transform(a);
        let pb = transform(b);
        let pc = transform(c);
        let pd = transform(d);
        let cross = (pb - pa).cross(pd - pa);
        let (face_positions, face_uvs) = if cross.dot(n) < 0.0 {
            ([pa, pb, pd, pc], [uvs[0], uvs[1], uvs[3], uvs[2]])
        } else {
            ([pa, pb, pc, pd], [uvs[0], uvs[1], uvs[2], uvs[3]])
        };
        let start_idx = vertices.len() as u32;
        for (pos, uv) in face_positions.iter().zip(face_uvs.iter()) {
            vertices.push(Vertex {
                position: [pos.x, pos.y, pos.z],
                tex_coords: [uv[0] / tex_w, uv[1] / tex_h],
                normal: [n.x, n.y, n.z],
                color: [1.0, 1.0, 1.0, 1.0],
                light: [1.0, 1.0],
            });
        }
        indices.push(start_idx);
        indices.push(start_idx + 1);
        indices.push(start_idx + 2);
        indices.push(start_idx);
        indices.push(start_idx + 2);
        indices.push(start_idx + 3);
    }
    (vertices, indices)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorSlot {
    Head,
    Chest,
    Legs,
    Feet,
}
pub fn generate_armor_mesh(parts: &PlayerModelParts, slot: ArmorSlot) -> (Vec<Vertex>, Vec<u32>) {
    let cuboid_count = match slot {
        ArmorSlot::Head | ArmorSlot::Feet => 2,
        ArmorSlot::Chest | ArmorSlot::Legs => 3,
    };
    let mut vertices = Vec::with_capacity(cuboid_count * 24);
    let mut indices = Vec::with_capacity(cuboid_count * 36);
    let body_rot = java_model_part_rotation(parts.body_pitch, parts.body_yaw, 0.0);
    let head_rot = java_model_part_rotation(parts.head_pitch, parts.head_yaw, 0.0);
    let right_arm_rot = java_model_part_rotation(
        parts.right_arm_pitch,
        parts.right_arm_yaw,
        parts.right_arm_roll,
    );
    let left_arm_rot = java_model_part_rotation(
        parts.left_arm_pitch,
        parts.left_arm_yaw,
        parts.left_arm_roll,
    );
    let right_leg_rot = java_model_part_rotation(
        parts.right_leg_pitch,
        parts.right_leg_yaw,
        parts.right_leg_roll,
    );
    let left_leg_rot = java_model_part_rotation(
        parts.left_leg_pitch,
        parts.left_leg_yaw,
        parts.left_leg_roll,
    );
    let y_dir = -1.0;
    let z_dir = -1.0;
    let outer = 1.0;
    let leggings = 0.4;
    let body_pivot = Vec3::new(0.0, 24.0 + parts.body_y_offset * y_dir, 0.0);
    let head_pivot = Vec3::new(0.0, 24.0 + parts.head_y_offset * y_dir, 0.0);
    let right_arm_pivot = Vec3::new(
        -5.0 + parts.right_arm_x_offset,
        22.0 + parts.right_arm_y_offset * y_dir,
        parts.right_arm_z_offset * z_dir,
    );
    let left_arm_pivot = Vec3::new(
        5.0 + parts.left_arm_x_offset,
        22.0 + parts.left_arm_y_offset * y_dir,
        parts.left_arm_z_offset * z_dir,
    );
    let right_leg_pivot = Vec3::new(-1.9, 12.0, parts.right_leg_z_offset * z_dir);
    let left_leg_pivot = Vec3::new(1.9, 12.0, parts.left_leg_z_offset * z_dir);
    match slot {
        ArmorSlot::Head => {
            add_armor_cuboid(
                &mut vertices,
                &mut indices,
                Vec3::new(
                    -4.0 - outer,
                    24.0 + parts.head_y_offset * y_dir - outer,
                    -4.0 - outer,
                ),
                Vec3::splat(8.0 + outer * 2.0),
                head_rot,
                head_pivot,
                [0.0, 0.0],
                [8.0, 8.0, 8.0],
            );
            add_armor_cuboid(
                &mut vertices,
                &mut indices,
                Vec3::new(
                    -4.0 - outer - 0.5,
                    24.0 + parts.head_y_offset * y_dir - outer - 0.5,
                    -4.0 - outer - 0.5,
                ),
                Vec3::splat(8.0 + (outer + 0.5) * 2.0),
                head_rot,
                head_pivot,
                [32.0, 0.0],
                [8.0, 8.0, 8.0],
            );
        }
        ArmorSlot::Chest => {
            add_armor_cuboid(
                &mut vertices,
                &mut indices,
                Vec3::new(
                    -4.0 - outer,
                    12.0 + parts.body_y_offset * y_dir - outer,
                    -2.0 - outer,
                ),
                Vec3::new(8.0 + outer * 2.0, 12.0 + outer * 2.0, 4.0 + outer * 2.0),
                body_rot,
                body_pivot,
                [16.0, 16.0],
                [8.0, 12.0, 4.0],
            );
            add_armor_cuboid(
                &mut vertices,
                &mut indices,
                Vec3::new(
                    -9.0 + parts.right_arm_x_offset,
                    11.0 + parts.right_arm_y_offset * y_dir,
                    -3.0 + parts.right_arm_z_offset * z_dir,
                ),
                Vec3::new(6.0, 14.0, 6.0),
                right_arm_rot,
                right_arm_pivot,
                [40.0, 16.0],
                [4.0, 12.0, 4.0],
            );
            add_armor_cuboid(
                &mut vertices,
                &mut indices,
                Vec3::new(
                    3.0 + parts.left_arm_x_offset,
                    11.0 + parts.left_arm_y_offset * y_dir,
                    -3.0 + parts.left_arm_z_offset * z_dir,
                ),
                Vec3::new(6.0, 14.0, 6.0),
                left_arm_rot,
                left_arm_pivot,
                [40.0, 16.0],
                [4.0, 12.0, 4.0],
            );
        }
        ArmorSlot::Legs => {
            add_armor_cuboid(
                &mut vertices,
                &mut indices,
                Vec3::new(
                    -4.0 - leggings,
                    12.0 + parts.body_y_offset * y_dir - leggings,
                    -2.0 - leggings,
                ),
                Vec3::new(
                    8.0 + leggings * 2.0,
                    12.0 + leggings * 2.0,
                    4.0 + leggings * 2.0,
                ),
                body_rot,
                body_pivot,
                [16.0, 16.0],
                [8.0, 12.0, 4.0],
            );
            add_armor_cuboid(
                &mut vertices,
                &mut indices,
                Vec3::new(
                    -3.9 - leggings,
                    -leggings,
                    -2.0 - leggings + parts.right_leg_z_offset * z_dir,
                ),
                Vec3::new(
                    4.0 + leggings * 2.0,
                    12.0 + leggings * 2.0,
                    4.0 + leggings * 2.0,
                ),
                right_leg_rot,
                right_leg_pivot,
                [0.0, 16.0],
                [4.0, 12.0, 4.0],
            );
            add_armor_cuboid(
                &mut vertices,
                &mut indices,
                Vec3::new(
                    -0.1 - leggings,
                    -leggings,
                    -2.0 - leggings + parts.left_leg_z_offset * z_dir,
                ),
                Vec3::new(
                    4.0 + leggings * 2.0,
                    12.0 + leggings * 2.0,
                    4.0 + leggings * 2.0,
                ),
                left_leg_rot,
                left_leg_pivot,
                [0.0, 16.0],
                [4.0, 12.0, 4.0],
            );
        }
        ArmorSlot::Feet => {
            for (origin, pivot, rotation) in [
                (
                    Vec3::new(
                        -3.9 - outer,
                        -outer,
                        -2.0 - outer + parts.right_leg_z_offset * z_dir,
                    ),
                    right_leg_pivot,
                    right_leg_rot,
                ),
                (
                    Vec3::new(
                        -0.1 - outer,
                        -outer,
                        -2.0 - outer + parts.left_leg_z_offset * z_dir,
                    ),
                    left_leg_pivot,
                    left_leg_rot,
                ),
            ] {
                add_armor_cuboid(
                    &mut vertices,
                    &mut indices,
                    origin,
                    Vec3::new(6.0, 14.0, 6.0),
                    rotation,
                    pivot,
                    [0.0, 16.0],
                    [4.0, 12.0, 4.0],
                );
            }
        }
    }
    (vertices, indices)
}
fn add_armor_cuboid(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    origin: Vec3,
    size: Vec3,
    rot: Quat,
    pivot: Vec3,
    uv: [f32; 2],
    uv_size: [f32; 3],
) {
    let px = 1.0 / 16.0;
    let (u, v) = (uv[0], uv[1]);
    let (dx, dy, dz) = (uv_size[0], uv_size[1], uv_size[2]);
    let ox = origin.x * px;
    let oy = origin.y * px;
    let oz = origin.z * px;
    let sx = size.x * px;
    let sy = size.y * px;
    let sz = size.z * px;
    add_quad_3d(
        vertices,
        indices,
        Vec3::new(ox, oy + sy, oz + sz),
        Vec3::new(ox, oy, oz + sz),
        Vec3::new(ox + sx, oy, oz + sz),
        Vec3::new(ox + sx, oy + sy, oz + sz),
        rot,
        pivot * px,
        [u + dz, v + dz],
        [dx, dy],
        64.0,
        32.0,
        Vec3::Z,
    );
    add_quad_3d(
        vertices,
        indices,
        Vec3::new(ox + sx, oy + sy, oz),
        Vec3::new(ox + sx, oy, oz),
        Vec3::new(ox, oy, oz),
        Vec3::new(ox, oy + sy, oz),
        rot,
        pivot * px,
        [u + dz + dx + dz, v + dz],
        [dx, dy],
        64.0,
        32.0,
        -Vec3::Z,
    );
    add_quad_3d(
        vertices,
        indices,
        Vec3::new(ox, oy + sy, oz),
        Vec3::new(ox, oy + sy, oz + sz),
        Vec3::new(ox + sx, oy + sy, oz + sz),
        Vec3::new(ox + sx, oy + sy, oz),
        rot,
        pivot * px,
        [u + dz, v],
        [dx, dz],
        64.0,
        32.0,
        Vec3::Y,
    );
    add_quad_3d(
        vertices,
        indices,
        Vec3::new(ox, oy, oz + sz),
        Vec3::new(ox, oy, oz),
        Vec3::new(ox + sx, oy, oz),
        Vec3::new(ox + sx, oy, oz + sz),
        rot,
        pivot * px,
        [u + dz + dx, v],
        [dx, dz],
        64.0,
        32.0,
        -Vec3::Y,
    );
    add_quad_3d(
        vertices,
        indices,
        Vec3::new(ox, oy + sy, oz),
        Vec3::new(ox, oy, oz),
        Vec3::new(ox, oy, oz + sz),
        Vec3::new(ox, oy + sy, oz + sz),
        rot,
        pivot * px,
        [u, v + dz],
        [dz, dy],
        64.0,
        32.0,
        -Vec3::X,
    );
    add_quad_3d(
        vertices,
        indices,
        Vec3::new(ox + sx, oy + sy, oz + sz),
        Vec3::new(ox + sx, oy, oz + sz),
        Vec3::new(ox + sx, oy, oz),
        Vec3::new(ox + sx, oy + sy, oz),
        rot,
        pivot * px,
        [u + dz + dx, v + dz],
        [dz, dy],
        64.0,
        32.0,
        Vec3::X,
    );
}
pub fn generate_first_person_arm(
    slim: bool,
    hand: HandSide,
    show_sleeve: bool,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let px = 1.0 / 16.0;
    let tex_w = 64.0;
    let tex_h = 64.0;
    let arm_w = if slim { 3.0 } else { 4.0 };
    let size = Vec3::new(arm_w, 12.0, 4.0);
    let (pivot, local_origin, rot, uv_arm, uv_sleeve) = match hand {
        HandSide::Right => (
            Vec3::new(-5.0, 2.0, 0.0),
            if slim {
                Vec3::new(-2.0, -2.0, -2.0)
            } else {
                Vec3::new(-3.0, -2.0, -2.0)
            },
            Quat::from_rotation_z(-0.1),
            [40.0, 16.0],
            [40.0, 32.0],
        ),
        HandSide::Left => (
            Vec3::new(5.0, 2.0, 0.0),
            Vec3::new(-1.0, -2.0, -2.0),
            Quat::from_rotation_z(0.1),
            [32.0, 48.0],
            [48.0, 48.0],
        ),
    };
    let origin = pivot + local_origin;
    let mut add_cuboid =
        |origin: Vec3, size: Vec3, rot: Quat, pivot: Vec3, uv: [f32; 2], uv_size: [f32; 3]| {
            let (u, v) = (uv[0], uv[1]);
            let (dx, dy, dz) = (uv_size[0], uv_size[1], uv_size[2]);
            let sx = size.x * px;
            let sy = size.y * px;
            let sz = size.z * px;
            let ox = origin.x * px;
            let oy = origin.y * px;
            let oz = origin.z * px;
            let px_pivot = pivot * px;
            add_quad_3d_v_inverted(
                &mut vertices,
                &mut indices,
                Vec3::new(ox, oy + sy, oz + sz),
                Vec3::new(ox, oy, oz + sz),
                Vec3::new(ox + sx, oy, oz + sz),
                Vec3::new(ox + sx, oy + sy, oz + sz),
                rot,
                px_pivot,
                [u + dz, v + dz],
                [dx, dy],
                tex_w,
                tex_h,
                Vec3::Z,
            );
            add_quad_3d_v_inverted(
                &mut vertices,
                &mut indices,
                Vec3::new(ox + sx, oy + sy, oz),
                Vec3::new(ox + sx, oy, oz),
                Vec3::new(ox, oy, oz),
                Vec3::new(ox, oy + sy, oz),
                rot,
                px_pivot,
                [u + dz + dx + dz, v + dz],
                [dx, dy],
                tex_w,
                tex_h,
                -Vec3::Z,
            );
            add_quad_3d(
                &mut vertices,
                &mut indices,
                Vec3::new(ox, oy + sy, oz),
                Vec3::new(ox, oy + sy, oz + sz),
                Vec3::new(ox + sx, oy + sy, oz + sz),
                Vec3::new(ox + sx, oy + sy, oz),
                rot,
                px_pivot,
                [u + dz, v],
                [dx, dz],
                tex_w,
                tex_h,
                Vec3::Y,
            );
            add_quad_3d(
                &mut vertices,
                &mut indices,
                Vec3::new(ox, oy, oz + sz),
                Vec3::new(ox, oy, oz),
                Vec3::new(ox + sx, oy, oz),
                Vec3::new(ox + sx, oy, oz + sz),
                rot,
                px_pivot,
                [u + dz + dx, v],
                [dx, dz],
                tex_w,
                tex_h,
                -Vec3::Y,
            );
            add_quad_3d_v_inverted(
                &mut vertices,
                &mut indices,
                Vec3::new(ox, oy + sy, oz),
                Vec3::new(ox, oy, oz),
                Vec3::new(ox, oy, oz + sz),
                Vec3::new(ox, oy + sy, oz + sz),
                rot,
                px_pivot,
                [u, v + dz],
                [dz, dy],
                tex_w,
                tex_h,
                -Vec3::X,
            );
            add_quad_3d_v_inverted(
                &mut vertices,
                &mut indices,
                Vec3::new(ox + sx, oy + sy, oz + sz),
                Vec3::new(ox + sx, oy, oz + sz),
                Vec3::new(ox + sx, oy, oz),
                Vec3::new(ox + sx, oy + sy, oz),
                rot,
                px_pivot,
                [u + dz + dx, v + dz],
                [dz, dy],
                tex_w,
                tex_h,
                Vec3::X,
            );
        };
    add_cuboid(origin, size, rot, pivot, uv_arm, [arm_w, 12.0, 4.0]);
    if show_sleeve {
        let sleeve_size = Vec3::new(arm_w + 0.5, 12.0 + 0.5, 4.0 + 0.5);
        let sleeve_origin = origin - Vec3::new(0.25, 0.25, 0.25);
        add_cuboid(
            sleeve_origin,
            sleeve_size,
            rot,
            pivot,
            uv_sleeve,
            [arm_w, 12.0, 4.0],
        );
    }
    (vertices, indices)
}
fn add_quad_3d(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    v3: Vec3,
    rot: Quat,
    pivot: Vec3,
    uv: [f32; 2],
    uv_size: [f32; 2],
    tex_w: f32,
    tex_h: f32,
    normal: Vec3,
) {
    let start_idx = vertices.len() as u32;
    let p0 = pivot + rot * (v0 - pivot);
    let p1 = pivot + rot * (v1 - pivot);
    let p2 = pivot + rot * (v2 - pivot);
    let p3 = pivot + rot * (v3 - pivot);
    let n = rot * normal;
    let normal_arr = [n.x, n.y, n.z];
    let color = [1.0, 1.0, 1.0, 1.0];
    let u0 = uv[0] / tex_w;
    let v0 = uv[1] / tex_h;
    let u1 = (uv[0] + uv_size[0]) / tex_w;
    let v1 = (uv[1] + uv_size[1]) / tex_h;
    vertices.push(Vertex {
        position: [p0.x, p0.y, p0.z],
        tex_coords: [u0, v0],
        normal: normal_arr,
        color,
        light: [1.0, 1.0],
    });
    vertices.push(Vertex {
        position: [p1.x, p1.y, p1.z],
        tex_coords: [u0, v1],
        normal: normal_arr,
        color,
        light: [1.0, 1.0],
    });
    vertices.push(Vertex {
        position: [p2.x, p2.y, p2.z],
        tex_coords: [u1, v1],
        normal: normal_arr,
        color,
        light: [1.0, 1.0],
    });
    vertices.push(Vertex {
        position: [p3.x, p3.y, p3.z],
        tex_coords: [u1, v0],
        normal: normal_arr,
        color,
        light: [1.0, 1.0],
    });
    indices.push(start_idx);
    indices.push(start_idx + 1);
    indices.push(start_idx + 2);
    indices.push(start_idx);
    indices.push(start_idx + 2);
    indices.push(start_idx + 3);
}
fn add_quad_3d_v_inverted(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    v0: Vec3,
    v1: Vec3,
    v2: Vec3,
    v3: Vec3,
    rot: Quat,
    pivot: Vec3,
    uv: [f32; 2],
    uv_size: [f32; 2],
    tex_w: f32,
    tex_h: f32,
    normal: Vec3,
) {
    let start_idx = vertices.len() as u32;
    let p0 = pivot + rot * (v0 - pivot);
    let p1 = pivot + rot * (v1 - pivot);
    let p2 = pivot + rot * (v2 - pivot);
    let p3 = pivot + rot * (v3 - pivot);
    let n = rot * normal;
    let normal_arr = [n.x, n.y, n.z];
    let color = [1.0, 1.0, 1.0, 1.0];
    let u0 = uv[0] / tex_w;
    let v0 = uv[1] / tex_h;
    let u1 = (uv[0] + uv_size[0]) / tex_w;
    let v1 = (uv[1] + uv_size[1]) / tex_h;
    vertices.push(Vertex {
        position: [p0.x, p0.y, p0.z],
        tex_coords: [u0, v1],
        normal: normal_arr,
        color,
        light: [1.0, 1.0],
    });
    vertices.push(Vertex {
        position: [p1.x, p1.y, p1.z],
        tex_coords: [u0, v0],
        normal: normal_arr,
        color,
        light: [1.0, 1.0],
    });
    vertices.push(Vertex {
        position: [p2.x, p2.y, p2.z],
        tex_coords: [u1, v0],
        normal: normal_arr,
        color,
        light: [1.0, 1.0],
    });
    vertices.push(Vertex {
        position: [p3.x, p3.y, p3.z],
        tex_coords: [u1, v1],
        normal: normal_arr,
        color,
        light: [1.0, 1.0],
    });
    indices.push(start_idx);
    indices.push(start_idx + 1);
    indices.push(start_idx + 2);
    indices.push(start_idx);
    indices.push(start_idx + 2);
    indices.push(start_idx + 3);
}
