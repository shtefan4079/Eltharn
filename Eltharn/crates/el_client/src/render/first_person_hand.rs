use super::*;
use crate::render::item_in_hand_renderer::ItemUseAnimation;
use crate::render::model::player::player_model::HandSide;
use el_core::item::ItemStack;
pub(in crate::client::state) struct FirstPersonHandInputs<'a> {
    pub player_texture: &'a wgpu::BindGroup,
    pub aspect: f32,
    pub hud_fov_radians: f32,
    pub partial_tick: f32,
    pub view_bobbing: bool,
    pub backwards_walk_dist: f32,
    pub bob: f32,
    pub view_pitch: f32,
    pub view_yaw: f32,
    pub x_bob: f32,
    pub y_bob: f32,
    pub first_person_light: [f32; 2],
    pub main_hand_side: HandSide,
    pub off_hand_side: HandSide,
    pub selection_render_main_hand: bool,
    pub selection_render_off_hand: bool,
    pub use_hand_physical: Option<HandSide>,
    pub use_animation: Option<ItemUseAnimation>,
    pub use_duration: f32,
    pub use_remaining: f32,
    pub player_is_using_item: bool,
    pub player_invisible: bool,
    pub swinging_hand: HandSide,
    pub attack_anim: f32,
    pub main_height: f32,
    pub off_height: f32,
    pub main_swap_animation_scale: f32,
    pub off_swap_animation_scale: f32,
    pub main_stack: &'a ItemStack,
    pub off_stack: &'a ItemStack,
    pub map_offhand_empty: bool,
    pub crossbow_duration_main: f32,
    pub crossbow_duration_off: f32,
    pub kinetic_feedback_ticks: f32,
}
fn first_person_item_camera_uniform(
    mut frame: CameraUniform,
    projection: glam::Mat4,
    item_pose: glam::Mat4,
    light: [f32; 2],
) -> CameraUniform {
    frame.overlay_color = [1.0; 4];
    frame.view_proj = (projection * item_pose).to_cols_array_2d();
    apply_cached_mesh_lightmap_override(&mut frame, light);
    rebase_model_space_fog_origin(
        &mut frame,
        glam::Vec3::ZERO,
        item_pose.transform_point3(glam::Vec3::ZERO),
    );
    frame
}
impl ClientRenderer {
    pub(in crate::client::state) fn encode_first_person_hand(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        input: FirstPersonHandInputs<'_>,
        counters: Option<&mut crate::render::profiler::RenderEncodeScopeCounters>,
    ) {
        let atlas = self.block_atlas.as_ref().expect("prepared atlas");
        let scene_view = &self.scene_texture_view;
        let proj = java_hud_projection_like_java(input.hud_fov_radians, input.aspect);
        let partial_tick = input.partial_tick;
        let view_bob = if input.view_bobbing {
            crate::render::model::player::player_model::vanilla_view_bob_matrix(
                input.backwards_walk_dist,
                input.bob,
            )
        } else {
            glam::Mat4::IDENTITY
        };
        let view_pitch = input.view_pitch;
        let view_yaw = input.view_yaw;
        let first_person_light = input.first_person_light;
        let hand_camera_bob =
            crate::render::model::player::player_model::vanilla_hand_camera_bob_matrix(
                view_pitch,
                view_yaw,
                input.x_bob,
                input.y_bob,
            );
        let main_hand_side = input.main_hand_side;
        let off_hand_side = input.off_hand_side;
        let player_tex = input.player_texture;
        let mut render_pass = CountedRenderPass::new(
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FP Arm Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: scene_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(JAVA_REVERSE_DEPTH_CLEAR_VALUE),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            }),
            counters,
        );
        let mut hands_to_render = Vec::new();
        if input.selection_render_main_hand {
            hands_to_render.push(main_hand_side);
        }
        if input.selection_render_off_hand {
            hands_to_render.push(off_hand_side);
        }
        for hand in hands_to_render {
            let is_main_hand = hand == main_hand_side;
            let item_stack: &ItemStack = if is_main_hand {
                input.main_stack
            } else {
                input.off_stack
            };
            let inferred_use_data = input
                .use_animation
                .map(|animation| (animation, input.use_duration))
                .or_else(|| {
                    input
                        .player_is_using_item
                        .then(|| {
                            item_in_hand_renderer::get_item_use_animation_and_duration(item_stack)
                        })
                        .flatten()
                });
            let inferred_use = inferred_use_data.map(|(animation, _)| animation);
            let inferred_use_duration = if input.use_duration > 0.0 {
                input.use_duration
            } else {
                inferred_use_data
                    .map(|(_, duration)| duration)
                    .unwrap_or(0.0)
            };
            let inferred_use_remaining = if input.use_duration > 0.0 {
                input.use_remaining
            } else {
                inferred_use_duration
            };
            let is_using_this_hand = input.use_hand_physical == Some(hand)
                || (input.use_hand_physical.is_none()
                    && input.player_is_using_item
                    && inferred_use.is_some());
            let is_right_arm = hand == crate::render::model::player::player_model::HandSide::Right;
            let invert = if is_right_arm { 1.0 } else { -1.0 };
            let attack_value = if hand == input.swinging_hand {
                input.attack_anim.clamp(0.0, 1.0)
            } else {
                0.0
            };
            let inverse_arm_height = if is_main_hand {
                input.main_swap_animation_scale * (1.0 - input.main_height)
            } else {
                input.off_swap_animation_scale * (1.0 - input.off_height)
            };
            let item_mesh_available = if is_main_hand {
                self.held_item_num_indices > 0
            } else {
                self.off_hand_item_num_indices > 0
            };
            let is_map = item_stack.item.location.path == "filled_map"
                || item_stack.item.location.path == "map";
            let crossbow_duration = if is_main_hand {
                input.crossbow_duration_main
            } else {
                input.crossbow_duration_off
            };
            let mut draw_arm = false;
            let mut arm_matrix = glam::Mat4::IDENTITY;
            let mut render_two_hands_map_arms = false;
            let mut map_matrix_two_handed = glam::Mat4::IDENTITY;
            if item_stack.is_empty() || !item_mesh_available {
                if is_main_hand && !input.player_invisible {
                    draw_arm = true;
                    arm_matrix = crate::render::model::player::player_model::vanilla_render_player_arm_matrix(
                        hand,
                        inverse_arm_height,
                        attack_value,
                    );
                }
            } else if is_map {
                let off_empty = input.map_offhand_empty;
                if is_main_hand && off_empty {
                    render_two_hands_map_arms = !input.player_invisible;
                    let sqrt_attack = attack_value.sqrt();
                    let y_swing_pos = -0.2 * (attack_value * std::f32::consts::PI).sin();
                    let z_swing_pos = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
                    let x_rot = view_pitch.to_degrees();
                    let map_tilt = item_in_hand_renderer::calculate_map_tilt(x_rot);
                    map_matrix_two_handed =
                        glam::Mat4::from_translation(glam::Vec3::new(
                            0.0,
                            -y_swing_pos / 2.0,
                            z_swing_pos,
                        )) * glam::Mat4::from_translation(glam::Vec3::new(
                            0.0,
                            0.04 + inverse_arm_height * -1.2 + map_tilt * -0.5,
                            -0.72,
                        )) * glam::Mat4::from_rotation_x((map_tilt * -85.0_f32).to_radians());
                } else {
                    draw_arm = !input.player_invisible;
                    arm_matrix = glam::Mat4::from_translation(
                        glam::Vec3::new(invert * 0.125, -0.125, 0.0),
                    ) * glam::Mat4::from_rotation_z((invert * 10.0_f32).to_radians())
                        * crate::render::model::player::player_model::vanilla_render_player_arm_matrix(
                            hand,
                            inverse_arm_height,
                            attack_value,
                        );
                }
            }
            if draw_arm {
                let mut player_camera_uniform = self.camera_uniform;
                player_camera_uniform.overlay_color = [1.0; 4];
                player_camera_uniform.view_proj =
                    (proj * view_bob * hand_camera_bob * arm_matrix).to_cols_array_2d();
                apply_cached_mesh_lightmap_override(&mut player_camera_uniform, first_person_light);
                let (c_buf, c_bg) = if is_right_arm {
                    (&self.player_camera_buffer, &self.player_camera_bind_group)
                } else {
                    (&self.fp_left_camera_buffer, &self.fp_left_camera_bind_group)
                };
                queue.write_buffer(c_buf, 0, bytemuck::cast_slice(&[player_camera_uniform]));
                render_pass.set_pipeline(&self.fp_arm_pipeline);
                render_pass.set_bind_group(0, c_bg, &[]);
                render_pass.set_bind_group(1, player_tex, &[]);
                let (v_buf, i_buf, num_idx) = if is_right_arm {
                    (
                        &self.fp_right_arm_vertex_buffer,
                        &self.fp_right_arm_index_buffer,
                        self.fp_right_arm_num_indices,
                    )
                } else {
                    (
                        &self.fp_left_arm_vertex_buffer,
                        &self.fp_left_arm_index_buffer,
                        self.fp_left_arm_num_indices,
                    )
                };
                if let (Some(vb), Some(ib)) = (v_buf, i_buf) {
                    render_pass.set_vertex_buffer(0, vb.slice(..));
                    render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..num_idx, 0, 0..1);
                }
            } else if render_two_hands_map_arms {
                let arm_matrix_r = map_matrix_two_handed
                    * glam::Mat4::from_rotation_y(90.0_f32.to_radians())
                    * glam::Mat4::from_rotation_y(92.0_f32.to_radians())
                    * glam::Mat4::from_rotation_x(45.0_f32.to_radians())
                    * glam::Mat4::from_rotation_z((-41.0_f32).to_radians())
                    * glam::Mat4::from_translation(glam::Vec3::new(0.3, -1.1, 0.45));
                let mut player_camera_uniform = self.camera_uniform;
                player_camera_uniform.overlay_color = [1.0; 4];
                player_camera_uniform.view_proj =
                    (proj * view_bob * hand_camera_bob * arm_matrix_r).to_cols_array_2d();
                apply_cached_mesh_lightmap_override(&mut player_camera_uniform, first_person_light);
                queue.write_buffer(
                    &self.player_camera_buffer,
                    0,
                    bytemuck::cast_slice(&[player_camera_uniform]),
                );
                render_pass.set_pipeline(&self.fp_arm_pipeline);
                render_pass.set_bind_group(0, &self.player_camera_bind_group, &[]);
                render_pass.set_bind_group(1, player_tex, &[]);
                if let (Some(vb), Some(ib)) = (
                    &self.fp_right_arm_vertex_buffer,
                    &self.fp_right_arm_index_buffer,
                ) {
                    render_pass.set_vertex_buffer(0, vb.slice(..));
                    render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..self.fp_right_arm_num_indices, 0, 0..1);
                }
                let arm_matrix_l = map_matrix_two_handed
                    * glam::Mat4::from_rotation_y(90.0_f32.to_radians())
                    * glam::Mat4::from_rotation_y(92.0_f32.to_radians())
                    * glam::Mat4::from_rotation_x(45.0_f32.to_radians())
                    * glam::Mat4::from_rotation_z(41.0_f32.to_radians())
                    * glam::Mat4::from_translation(glam::Vec3::new(-0.3, -1.1, 0.45));
                let mut player_camera_uniform_l = self.camera_uniform;
                player_camera_uniform_l.overlay_color = [1.0; 4];
                player_camera_uniform_l.view_proj =
                    (proj * view_bob * hand_camera_bob * arm_matrix_l).to_cols_array_2d();
                apply_cached_mesh_lightmap_override(
                    &mut player_camera_uniform_l,
                    first_person_light,
                );
                queue.write_buffer(
                    &self.fp_left_camera_buffer,
                    0,
                    bytemuck::cast_slice(&[player_camera_uniform_l]),
                );
                render_pass.set_pipeline(&self.fp_arm_pipeline);
                render_pass.set_bind_group(0, &self.fp_left_camera_bind_group, &[]);
                render_pass.set_bind_group(1, player_tex, &[]);
                if let (Some(vb), Some(ib)) = (
                    &self.fp_left_arm_vertex_buffer,
                    &self.fp_left_arm_index_buffer,
                ) {
                    render_pass.set_vertex_buffer(0, vb.slice(..));
                    render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..self.fp_left_arm_num_indices, 0, 0..1);
                }
            }
            let item_v_buf = if is_main_hand {
                &self.held_item_vertex_buffer
            } else {
                &self.off_hand_item_vertex_buffer
            };
            let item_i_buf = if is_main_hand {
                &self.held_item_index_buffer
            } else {
                &self.off_hand_item_index_buffer
            };
            let item_num_idx = if is_main_hand {
                self.held_item_num_indices
            } else {
                self.off_hand_item_num_indices
            };
            if item_num_idx > 0 {
                if let (Some(vb), Some(ib)) = (item_v_buf, item_i_buf) {
                    let mut item_matrix = glam::Mat4::IDENTITY;
                    if is_map {
                        if is_main_hand && input.map_offhand_empty {
                            let sqrt_attack = attack_value.sqrt();
                            let y_swing_pos = -0.2 * (attack_value * std::f32::consts::PI).sin();
                            let z_swing_pos = -0.4 * (sqrt_attack * std::f32::consts::PI).sin();
                            let x_rot = view_pitch.to_degrees();
                            let map_tilt = item_in_hand_renderer::calculate_map_tilt(x_rot);
                            let map_matrix = glam::Mat4::from_translation(glam::Vec3::new(
                                0.0,
                                -y_swing_pos / 2.0,
                                z_swing_pos,
                            )) * glam::Mat4::from_translation(glam::Vec3::new(
                                0.0,
                                0.04 + inverse_arm_height * -1.2 + map_tilt * -0.5,
                                -0.72,
                            )) * glam::Mat4::from_rotation_x(
                                (map_tilt * -85.0_f32).to_radians(),
                            );
                            item_matrix = map_matrix
                                * glam::Mat4::from_rotation_y(180.0_f32.to_radians())
                                * glam::Mat4::from_rotation_z(180.0_f32.to_radians())
                                * glam::Mat4::from_scale(glam::Vec3::splat(0.38))
                                * glam::Mat4::from_translation(glam::Vec3::new(-0.5, -0.5, 0.0))
                                * glam::Mat4::from_scale(glam::Vec3::splat(0.0078125));
                        } else {
                            let base_matrix = glam::Mat4::from_translation(glam::Vec3::new(
                                invert * 0.125,
                                -0.125,
                                0.0,
                            )) * glam::Mat4::from_translation(glam::Vec3::new(
                                invert * 0.51,
                                -0.08 + inverse_arm_height * -1.2,
                                -0.75,
                            ));
                            let sqrt_attack = attack_value.sqrt();
                            let x_swing = (sqrt_attack * std::f32::consts::PI).sin();
                            let x_swing_pos = -0.5 * x_swing;
                            let y_swing_pos =
                                0.4 * (sqrt_attack * std::f32::consts::PI * 2.0).sin();
                            let z_swing_pos = -0.3 * (attack_value * std::f32::consts::PI).sin();
                            item_matrix = base_matrix
                                * glam::Mat4::from_translation(glam::Vec3::new(
                                    invert * x_swing_pos,
                                    y_swing_pos - 0.3 * x_swing,
                                    z_swing_pos,
                                ))
                                * glam::Mat4::from_rotation_x((x_swing * -45.0_f32).to_radians())
                                * glam::Mat4::from_rotation_y(
                                    (invert * x_swing * -30.0_f32).to_radians(),
                                )
                                * glam::Mat4::from_rotation_y(180.0_f32.to_radians())
                                * glam::Mat4::from_rotation_z(180.0_f32.to_radians())
                                * glam::Mat4::from_scale(glam::Vec3::splat(0.38))
                                * glam::Mat4::from_translation(glam::Vec3::new(-0.5, -0.5, 0.0))
                                * glam::Mat4::from_scale(glam::Vec3::splat(0.0078125));
                        }
                    } else if item_stack.item.location.path == "crossbow" {
                        item_matrix = glam::Mat4::from_translation(glam::Vec3::new(
                            invert * 0.56,
                            -0.52 + inverse_arm_height * -0.6,
                            -0.72,
                        ));
                        let crossbow_charged =
                            item_in_hand_renderer::is_charged_crossbow(item_stack);
                        if is_using_this_hand
                            && inferred_use_remaining > 0.0
                            && inferred_use == Some(ItemUseAnimation::Crossbow)
                            && !crossbow_charged
                        {
                            item_matrix *= item_in_hand_renderer::crossbow_use_transform_matrix(
                                hand,
                                partial_tick,
                                inferred_use_remaining,
                                inferred_use_duration,
                                crossbow_duration,
                            );
                        } else {
                            item_matrix *=
                                item_in_hand_renderer::swing_arm_matrix(hand, attack_value);
                            if crossbow_charged {
                                item_matrix *= item_in_hand_renderer::charged_crossbow_hold_matrix(
                                    hand,
                                    is_main_hand,
                                    attack_value,
                                );
                            }
                        }
                    } else {
                        let is_using_this = is_using_this_hand
                            && (inferred_use_remaining > 0.0
                                || input.use_hand_physical == Some(hand));
                        let has_custom = is_using_this
                            && inferred_use
                                .as_ref()
                                .copied()
                                .is_some_and(ItemUseAnimation::has_custom_arm_transform);
                        if !has_custom {
                            item_matrix = glam::Mat4::from_translation(glam::Vec3::new(
                                invert * 0.56,
                                -0.52 + inverse_arm_height * -0.6,
                                -0.72,
                            ));
                        }
                        if is_using_this {
                            let render_use_remaining = if input.use_hand_physical == Some(hand) {
                                inferred_use_remaining.max(0.0)
                            } else {
                                inferred_use_remaining
                            };
                            match inferred_use {
                                Some(ItemUseAnimation::Eat) | Some(ItemUseAnimation::Drink) => {
                                    item_matrix = item_matrix
                                        * item_in_hand_renderer::eat_transform_matrix(
                                            hand,
                                            partial_tick,
                                            render_use_remaining,
                                            inferred_use_duration,
                                        )
                                        * item_in_hand_renderer::apply_item_arm_transform(
                                            hand,
                                            inverse_arm_height,
                                        );
                                }
                                Some(ItemUseAnimation::Block) => {
                                    item_matrix = item_in_hand_renderer::item_use_transform_matrix(
                                        item_in_hand_renderer::HandUseContext {
                                            hand,
                                            inverse_arm_height,
                                            attack_value,
                                            frame_interp: partial_tick,
                                            use_remaining_ticks: inferred_use_remaining,
                                            use_duration: inferred_use_duration,
                                            charged_crossbow: false,
                                            crossbow_charge_duration: crossbow_duration,
                                        },
                                        item_stack,
                                        ItemUseAnimation::Block,
                                    );
                                }
                                Some(ItemUseAnimation::Bow) => {
                                    item_matrix = item_in_hand_renderer::item_use_transform_matrix(
                                        item_in_hand_renderer::HandUseContext {
                                            hand,
                                            inverse_arm_height,
                                            attack_value,
                                            frame_interp: partial_tick,
                                            use_remaining_ticks: inferred_use_remaining,
                                            use_duration: inferred_use_duration,
                                            charged_crossbow: false,
                                            crossbow_charge_duration: crossbow_duration,
                                        },
                                        item_stack,
                                        ItemUseAnimation::Bow,
                                    );
                                }
                                Some(ItemUseAnimation::Trident) => {
                                    item_matrix = item_in_hand_renderer::item_use_transform_matrix(
                                        item_in_hand_renderer::HandUseContext {
                                            hand,
                                            inverse_arm_height,
                                            attack_value,
                                            frame_interp: partial_tick,
                                            use_remaining_ticks: inferred_use_remaining,
                                            use_duration: inferred_use_duration,
                                            charged_crossbow: false,
                                            crossbow_charge_duration: crossbow_duration,
                                        },
                                        item_stack,
                                        ItemUseAnimation::Trident,
                                    );
                                }
                                Some(ItemUseAnimation::Spear) => {
                                    item_matrix = glam::Mat4::from_translation(glam::Vec3::new(
                                        invert * 0.56,
                                        -0.52,
                                        -0.72,
                                    ));
                                    if let Some(kinetic_weapon) = item_stack.kinetic_weapon() {
                                        let time_held = inferred_use_duration
                                            - (inferred_use_remaining - partial_tick + 1.0);
                                        let feedback_ticks = input.kinetic_feedback_ticks;
                                        item_matrix *=
                                            item_in_hand_renderer::spear_first_person_use_matrix(
                                                hand,
                                                time_held,
                                                feedback_ticks,
                                                kinetic_weapon,
                                            );
                                    }
                                }
                                Some(ItemUseAnimation::Brush) => {
                                    let brush_rem = (inferred_use_remaining as i32) % 10;
                                    let scaled = 1.0 - (brush_rem as f32) / 10.0;
                                    let current_swipe =
                                        -15.0 + 75.0 * (scaled * 2.0 * std::f32::consts::PI).cos();
                                    if !is_right_arm {
                                        item_matrix = item_matrix
                                            * glam::Mat4::from_translation(glam::Vec3::new(
                                                0.1, 0.83, 0.35,
                                            ))
                                            * glam::Mat4::from_rotation_x(-80.0_f32.to_radians())
                                            * glam::Mat4::from_rotation_y(-90.0_f32.to_radians())
                                            * glam::Mat4::from_rotation_x(
                                                current_swipe.to_radians(),
                                            )
                                            * glam::Mat4::from_translation(glam::Vec3::new(
                                                -0.3, 0.22, 0.35,
                                            ));
                                    } else {
                                        item_matrix = item_matrix
                                            * glam::Mat4::from_translation(glam::Vec3::new(
                                                -0.25, 0.22, 0.35,
                                            ))
                                            * glam::Mat4::from_rotation_x(-80.0_f32.to_radians())
                                            * glam::Mat4::from_rotation_y(90.0_f32.to_radians())
                                            * glam::Mat4::from_rotation_x(
                                                current_swipe.to_radians(),
                                            );
                                    }
                                }
                                _ => {}
                            }
                        } else if let Some(swing_animation) = item_stack.swing_animation() {
                            item_matrix *= item_in_hand_renderer::swing_animation_matrix(
                                hand,
                                attack_value,
                                swing_animation.animation_type,
                            );
                        }
                    }
                    let item_camera_uniform = first_person_item_camera_uniform(
                        self.camera_uniform,
                        proj,
                        view_bob * hand_camera_bob * item_matrix,
                        first_person_light,
                    );
                    let (i_buf, i_bg) = if is_main_hand {
                        (
                            &self.held_item_camera_buffer,
                            &self.held_item_camera_bind_group,
                        )
                    } else {
                        (
                            &self.off_hand_item_camera_buffer,
                            &self.off_hand_item_camera_bind_group,
                        )
                    };
                    queue.write_buffer(i_buf, 0, bytemuck::cast_slice(&[item_camera_uniform]));
                    let custom_texture_key = if is_main_hand {
                        self.held_item_custom_texture_key.as_deref()
                    } else {
                        self.off_hand_item_custom_texture_key.as_deref()
                    };
                    let custom_head_texture = custom_texture_key
                        .and_then(|key| self.remote_entity_textures.get(key))
                        .is_some();
                    render_pass.set_pipeline(if custom_head_texture {
                        &self.inventory_preview_pipeline
                    } else {
                        &self.item_pipeline
                    });
                    render_pass.set_bind_group(0, i_bg, &[]);
                    if let Some(key) = custom_texture_key
                        && let Some(texture) = self.remote_entity_textures.get(key)
                    {
                        render_pass.set_bind_group(1, texture, &[]);
                    } else {
                        render_pass.set_bind_group(1, &atlas.bind_group, &[]);
                    }
                    render_pass.set_vertex_buffer(0, vb.slice(..));
                    render_pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.draw_indexed(0..item_num_idx, 0, 0..1);
                    let visible_stack = if is_main_hand {
                        self.held_item_stack.as_ref()
                    } else {
                        self.off_hand_item_stack.as_ref()
                    };
                    if visible_stack.is_some_and(item_has_foil) {
                        render_pass.set_pipeline(&self.item_glint_pipeline);
                        render_pass.set_bind_group(2, &self.item_glint_bind_group, &[]);
                        render_pass.set_bind_group(3, &self.item_glint_uniform_bind_group, &[]);
                        render_pass.set_bind_group(0, i_bg, &[]);
                        if let Some(key) = custom_texture_key
                            && let Some(texture) = self.remote_entity_textures.get(key)
                        {
                            render_pass.set_bind_group(1, texture, &[]);
                        } else {
                            render_pass.set_bind_group(1, &atlas.bind_group, &[]);
                        }
                        render_pass.draw_indexed(0..item_num_idx, 0, 0..1);
                    }
                }
            }
        }
    }
}
