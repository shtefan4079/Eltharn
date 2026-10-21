use super::*;
pub(super) enum PreparedFallbackEntity {
    ItemFrame {
        frame: Option<(Vec<Vertex>, Vec<u32>, CameraUniform)>,
        item: Option<(Vec<Vertex>, Vec<u32>, CameraUniform, bool, Option<String>)>,
    },
    DroppedItem {
        mesh_key: DroppedItemMeshKey,
        new_mesh: Option<(Vec<Vertex>, Vec<u32>, ItemMeshBounds)>,
        camera_uniforms: ([CameraUniform; 5], usize),
        has_foil: bool,
    },
    RemotePlayer {
        entity_id: i32,
        texture: Option<FallbackPlayerTextureSource>,
        body_visible: bool,
        layers_visible: bool,
        camera_uniform: CameraUniform,
        held_item_uniforms: [Option<(
            crate::render::model::player::player_model::HandSide,
            CameraUniform,
        )>; 2],
    },
    RemoteEntity {
        #[cfg(feature = "shaderpacks")]
        entity_id: i32,
        #[cfg(feature = "shaderpacks")]
        auxiliary_cull: Option<bool>,
        #[cfg(feature = "shaderpacks")]
        model_matrix: glam::Mat4,
        main_visible: bool,
        texture_path: &'static str,
        use_entity_cutout_pipeline: bool,
        dynamic_mesh: Option<(Vec<Vertex>, Vec<u32>)>,
        wool_mesh: Option<(Vec<Vertex>, Vec<u32>)>,
        armor_meshes: [Vec<(String, Vec<Vertex>, Vec<u32>, bool, bool)>; 4],
        camera_uniform: CameraUniform,
    },
}
pub(super) struct FallbackEntityPrepareContext<'a> {
    pub(super) remote_player_entity_id: Option<i32>,
    pub(super) remote_player_uuid: Option<uuid::Uuid>,
    pub(super) player_visibilities: &'a HashMap<i32, crate::render::model_frame::ModelVisibility>,
    pub(super) pretranslucent_remote_entity_ids: &'a HashSet<i32>,
    #[cfg(feature = "shaderpacks")]
    pub(super) shader_entity_frame: Option<&'a client_renderer::shader_entities::EntityFrame>,
    pub(super) level: Option<&'a ClientLevel>,
    pub(super) camera: &'a Camera,
    pub(super) entity_view_scale: f32,
    pub(super) partial_tick: f32,
    pub(super) terrain_frustum: crate::render::chunk::Frustum,
    pub(super) atlas: &'a TextureAtlas,
    pub(super) resource_manager: &'a crate::resources::manager::ClientResourceManager,
    pub(super) block_colors: Option<&'a crate::render::block_colors::ClientBlockColors>,
    pub(super) camera_uniform: CameraUniform,
    pub(super) entity_render_camera_origin_d: glam::DVec3,
    pub(super) entity_view_proj: glam::Mat4,
    pub(super) remote_entity_textures: &'a HashMap<String, wgpu::BindGroup>,
    pub(super) player_texture_is_some: bool,
    pub(super) remote_player_model_meshes: &'a HashMap<i32, RemotePlayerModelMesh>,
    pub(super) dropped_item_meshes: &'a HashMap<DroppedItemMeshKey, DroppedItemMesh>,
    pub(super) maps: &'a HashMap<u32, crate::render::map::MapData>,
    pub(super) frame_head_texture_keys: &'a HashMap<i32, String>,
}
pub(super) fn prepare_fallback_entity(
    ctx: &FallbackEntityPrepareContext<'_>,
    entity_id: i32,
    entity: &RemoteEntityRenderState,
) -> Option<PreparedFallbackEntity> {
    if Some(entity_id) == ctx.remote_player_entity_id {
        return None;
    }
    if entity.uuid.is_some() && entity.uuid == ctx.remote_player_uuid {
        return None;
    }
    if ctx.pretranslucent_remote_entity_ids.contains(&entity_id) {
        return None;
    }
    #[cfg(feature = "shaderpacks")]
    if ctx
        .shader_entity_frame
        .is_some_and(|f| f.ids.contains(&entity_id))
    {
        return None;
    }
    let entity_key = remote_entity_type_key(&entity.kind);
    if entity_key == "falling_block" || entity_key == "experience_orb" {
        return None;
    }
    let is_invisible = entity.shared_flag(5);
    let is_armor_stand = entity_key == "armor_stand";
    if matches!(entity_key, "item_frame" | "glow_item_frame") {
        return item_frame::prepare(ctx, entity);
    }
    if fallback_invisibility_hides_all_layers(entity) {
        return None;
    }
    if is_armor_stand {
        let is_marker = entity.armor_stand_client_flags & 0x10 != 0;
        if is_marker {
            return None;
        }
    }
    let main_distance_visible = remote_entity_should_render_at_distance_like_java(
        entity,
        ctx.camera.position,
        ctx.entity_view_scale,
        ctx.partial_tick,
    );
    let entity_position = entity.render_position(ctx.partial_tick);
    let main_visible = main_distance_visible
        && ctx.terrain_frustum.contains_aabb(
            entity_position - glam::Vec3::new(2.0, 3.0, 2.0),
            entity_position + glam::Vec3::new(2.0, 3.0, 2.0),
        );
    #[cfg(feature = "shaderpacks")]
    let auxiliary_cull = client_renderer::shader_entities::entity_cutout_cull(
        entity,
        remote_entity_model_animation_state(entity, ctx.partial_tick, ctx.level).is_baby,
    )
    .filter(|_| {
        ctx.shader_entity_frame
            .is_some_and(|f| f.candidates.contains(&entity_id))
    });
    #[cfg(feature = "shaderpacks")]
    let shadow_visible = auxiliary_cull.is_some()
        && ctx
            .shader_entity_frame
            .is_some_and(|f| f.shadow_candidate(entity_id));
    #[cfg(not(feature = "shaderpacks"))]
    let shadow_visible = false;
    if !main_visible && !shadow_visible {
        return None;
    }
    let thrown_item_spec = thrown_item_renderer_spec_like_java(&entity.kind);
    if is_remote_item_entity(&entity.kind) || thrown_item_spec.is_some() {
        let is_dropped_item = is_remote_item_entity(&entity.kind);
        let Some(ref item_stack) = entity.item_stack else {
            return None;
        };
        let item_loc = &item_stack.item.location;
        let item_mesh_key = dropped_item_mesh_key(item_loc, item_stack);
        let cached_mesh = ctx.dropped_item_meshes.get(&item_mesh_key);
        let mut new_mesh: Option<(Vec<Vertex>, Vec<u32>, ItemMeshBounds)> = None;
        let mesh_bounds = if let Some(mesh) = cached_mesh {
            mesh.bounds
        } else {
            log::debug!("[DroppedItem] Building mesh for {}", item_loc);
            let (vertices, indices) = generate_held_item_mesh_for_display(
                item_loc,
                ctx.atlas,
                ctx.resource_manager,
                crate::render::model::player::player_model::HandSide::Right,
                HeldItemDisplay::Ground,
                ctx.block_colors,
                Some(item_stack),
            );
            if indices.is_empty() {
                log::warn!("[DroppedItem] Empty mesh for {}", item_loc);
                return None;
            }
            let bounds = ItemMeshBounds::from_vertices(&vertices);
            new_mesh = Some((vertices, indices, bounds));
            bounds
        };
        let (item_model_matrix, entity_light, copy_offsets, copy_count) = if is_dropped_item {
            let age_f = entity.age_in_ticks as f32 + ctx.partial_tick;
            let bob = (age_f / 10.0 + entity.bob_offset).sin() * 0.1 + 0.1;
            let spin = age_f / 20.0 + entity.bob_offset;
            const MIN_HOVER: f32 = 0.0625;
            let item_position_d = entity.render_position_d(ctx.partial_tick);
            let item_position = item_position_d.as_vec3();
            let relative_item_position = camera_relative_entity_position_d_like_java(
                item_position_d,
                ctx.entity_render_camera_origin_d,
            );
            let trans = glam::Mat4::from_translation(
                relative_item_position
                    + glam::Vec3::new(0.0, bob - mesh_bounds.y_min() + MIN_HOVER, 0.0),
            );
            let rot = glam::Mat4::from_rotation_y(spin);
            let (offsets, count) = dropped_item_copy_offsets(item_stack, mesh_bounds);
            (
                trans * rot,
                remote_entity_light(ctx.level, &entity.kind, item_position),
                offsets,
                count,
            )
        } else {
            let Some(spec) = thrown_item_spec else {
                return None;
            };
            let Some(level) = ctx.level else {
                return None;
            };
            let position_d = entity.render_position_d(ctx.partial_tick);
            let position = position_d.as_vec3();
            let mut offsets = [glam::Mat4::IDENTITY; 5];
            offsets[0] = glam::Mat4::IDENTITY;
            (
                thrown_item_model_matrix_like_java(
                    camera_relative_entity_position_d_like_java(
                        position_d,
                        ctx.entity_render_camera_origin_d,
                    ),
                    ctx.camera.render_rotation(),
                    spec.scale,
                ),
                thrown_item_light_like_java(level, position, spec, entity.shared_flags),
                offsets,
                1,
            )
        };
        let mut camera_uniforms = [ctx.camera_uniform; 5];
        let base_view_proj = ctx.entity_view_proj * item_model_matrix;
        for i in 0..copy_count {
            let offset = copy_offsets[i];
            let offset_matrix = item_model_matrix * offset;
            let mut entity_camera_uniform = ctx.camera_uniform;
            apply_cached_mesh_lightmap_override(&mut entity_camera_uniform, entity_light);
            entity_camera_uniform.view_proj = (base_view_proj * offset).to_cols_array_2d();
            rebase_model_space_fog_origin(
                &mut entity_camera_uniform,
                glam::Vec3::ZERO,
                offset_matrix.transform_point3(glam::Vec3::ZERO),
            );
            camera_uniforms[i] = entity_camera_uniform;
        }
        return Some(PreparedFallbackEntity::DroppedItem {
            mesh_key: item_mesh_key,
            new_mesh,
            camera_uniforms: (camera_uniforms, copy_count),
            has_foil: item_has_foil(item_stack),
        });
    }
    if is_remote_player_entity(&entity.kind) {
        let player_texture = if let Some(uuid) = entity.uuid {
            let key = format!("player_skin:{uuid}");
            if ctx.remote_entity_textures.contains_key(&key) {
                Some(FallbackPlayerTextureSource::Remote(
                    std::borrow::Cow::Owned(key),
                ))
            } else {
                let default_path = default_player_skin_texture_path(uuid);
                if ctx.remote_entity_textures.contains_key(default_path) {
                    Some(FallbackPlayerTextureSource::Remote(
                        std::borrow::Cow::Borrowed(default_path),
                    ))
                } else if ctx.player_texture_is_some {
                    Some(FallbackPlayerTextureSource::Local)
                } else {
                    None
                }
            }
        } else if ctx.player_texture_is_some {
            Some(FallbackPlayerTextureSource::Local)
        } else {
            None
        };
        let raw_entity_position_d = entity.render_position_d(ctx.partial_tick);
        let raw_entity_position = raw_entity_position_d.as_vec3();
        let camera_relative_entity_position = camera_relative_entity_position_d_like_java(
            raw_entity_position_d,
            ctx.entity_render_camera_origin_d,
        );
        let mut rot = glam::Mat4::from_rotation_y(-entity.render_body_yaw(ctx.partial_tick));
        let swim_amount = entity.render_swim_amount(ctx.partial_tick);
        if swim_amount > 0.0 {
            let x_rot = entity.render_pitch(ctx.partial_tick).to_degrees();
            let in_water = remote_entity_intersects_water(entity, ctx.level);
            let x_angle = swim_body_pitch_degrees_like_java(swim_amount, x_rot, in_water);
            rot *= glam::Mat4::from_rotation_x(-x_angle.to_radians());
            if entity.pose == el_protocol_backend::BackendEntityPose::Swimming {
                rot *= glam::Mat4::from_translation(glam::Vec3::new(0.0, -1.0, -0.3));
            }
        }
        let player_model_matrix = crate::render::model_frame::world_avatar_body_matrix(
            camera_relative_entity_position,
            remote_avatar_render_offset_like_java(entity),
            rot,
            remote_living_scale_like_java(entity),
        );
        let mut entity_camera_uniform = ctx.camera_uniform;
        entity_camera_uniform.overlay_color =
            java_hurt_overlay_color(entity.hurt_time, entity.death_time);
        let entity_light = remote_entity_light(ctx.level, &entity.kind, raw_entity_position);
        apply_cached_mesh_lightmap_override(&mut entity_camera_uniform, entity_light);
        entity_camera_uniform.view_proj =
            (ctx.entity_view_proj * player_model_matrix).to_cols_array_2d();
        rebase_model_space_fog_origin(
            &mut entity_camera_uniform,
            glam::Vec3::ZERO,
            player_model_matrix.transform_point3(glam::Vec3::ZERO),
        );
        let item_matrices = {
            let Some(mesh) = ctx.remote_player_model_meshes.get(&entity_id) else {
                return None;
            };
            let mut matrices = [None, None];
            let mut count = 0;
            if mesh.main_hand_item.is_some() {
                matrices[count] = Some((
                    crate::render::model::player::player_model::HandSide::Right,
                    crate::render::model::player::player_model::third_person_item_in_hand_matrix(
                        &mesh.parts,
                        crate::render::model::player::player_model::HandSide::Right,
                    ),
                ));
                count += 1;
            }
            if mesh.off_hand_item.is_some() {
                matrices[count] = Some((
                    crate::render::model::player::player_model::HandSide::Left,
                    crate::render::model::player::player_model::third_person_item_in_hand_matrix(
                        &mesh.parts,
                        crate::render::model::player::player_model::HandSide::Left,
                    ),
                ));
                count += 1;
            }
            (matrices, count)
        };
        let mut held_item_uniforms = [None, None];
        for i in 0..item_matrices.1 {
            if let Some((hand, item_matrix)) = item_matrices.0[i] {
                let combined_matrix = player_model_matrix * item_matrix;
                let mut item_camera_uniform = ctx.camera_uniform;
                item_camera_uniform.overlay_color = [1.0; 4];
                apply_cached_mesh_lightmap_override(&mut item_camera_uniform, entity_light);
                item_camera_uniform.view_proj =
                    (ctx.entity_view_proj * combined_matrix).to_cols_array_2d();
                rebase_model_space_fog_origin(
                    &mut item_camera_uniform,
                    glam::Vec3::ZERO,
                    combined_matrix.transform_point3(glam::Vec3::ZERO),
                );
                held_item_uniforms[i] = Some((hand, item_camera_uniform));
            }
        }
        return Some(PreparedFallbackEntity::RemotePlayer {
            entity_id,
            texture: player_texture,
            body_visible: ctx
                .player_visibilities
                .get(&entity_id)
                .copied()
                .map(crate::render::model_frame::should_submit_textured_world_body)
                .unwrap_or_else(|| remote_player_fallback_body_visible(entity)),
            layers_visible: !ctx
                .player_visibilities
                .get(&entity_id)
                .is_some_and(|visibility| visibility.is_spectator),
            camera_uniform: entity_camera_uniform,
            held_item_uniforms,
        });
    }
    if !has_vanilla_registered_entity_renderer(&entity.kind) {
        return None;
    }
    let Some(texture_path) = remote_entity_texture_path_for_render(entity) else {
        return None;
    };
    let entity_position_d = entity.render_position_d(ctx.partial_tick);
    let entity_position = entity_position_d.as_vec3();
    let entity_light = remote_entity_light(ctx.level, &entity.kind, entity_position);
    let model_state = remote_entity_model_animation_state(entity, ctx.partial_tick, ctx.level);
    let model_kind = vanilla_entity_model_kind(&entity.kind);
    let use_entity_cutout_pipeline =
        uses_java_entity_cutout_pipeline(model_kind, model_state.is_baby);
    let mut dynamic_mesh: Option<(Vec<Vertex>, Vec<u32>)> = None;
    if !is_invisible {
        if let Some(model_kind) = model_kind {
            let (vertices, indices) = remote_entity_dynamic_mesh_like_java(
                entity,
                model_kind,
                model_state,
                ctx.partial_tick,
            );
            if !vertices.is_empty() && !indices.is_empty() {
                dynamic_mesh = Some((vertices, indices));
            }
        }
    }
    let mut wool_mesh: Option<(Vec<Vertex>, Vec<u32>)> = None;
    if main_visible
        && remote_entity_type_key_eq(&entity.kind, "sheep")
        && ctx
            .remote_entity_textures
            .contains_key("entity/sheep/sheep_wool")
    {
        let (vertices, indices) = build_sheep_wool_mesh_animated(model_state);
        if !vertices.is_empty() && !indices.is_empty() {
            wool_mesh = Some((vertices, indices));
        }
    }
    let mut armor_meshes: [Vec<(String, Vec<Vertex>, Vec<u32>, bool, bool)>; 4] =
        std::array::from_fn(|_| Vec::new());
    if main_visible && entity_key == "armor_stand" {
        let parts = armor_stand_model_parts(entity);
        let slots = [
            crate::render::model::player::player_model::ArmorSlot::Head,
            crate::render::model::player::player_model::ArmorSlot::Chest,
            crate::render::model::player::player_model::ArmorSlot::Legs,
            crate::render::model::player::player_model::ArmorSlot::Feet,
        ];
        let equipment = [
            &entity.head_equipment,
            &entity.chest_equipment,
            &entity.legs_equipment,
            &entity.feet_equipment,
        ];
        for (index, (slot, stack)) in slots.into_iter().zip(equipment).enumerate() {
            let (base_vertices, indices) =
                crate::render::model::player::player_model::generate_armor_mesh(&parts, slot);
            if !base_vertices.is_empty() && !indices.is_empty() {
                for layer in crate::render::model::equipment::resolve_armor_layers(
                    stack,
                    slot,
                    ctx.resource_manager,
                ) {
                    let mut vertices = base_vertices.clone();
                    for vertex in &mut vertices {
                        for (value, tint) in vertex.color.iter_mut().zip(layer.color) {
                            *value *= tint;
                        }
                    }
                    armor_meshes[index].push((
                        layer.texture_key,
                        vertices,
                        indices.clone(),
                        layer.has_foil,
                        layer.decal,
                    ));
                }
            }
        }
    }
    let mut entity_camera_uniform = ctx.camera_uniform;
    entity_camera_uniform.overlay_color =
        java_hurt_overlay_color(entity.hurt_time, entity.death_time);
    apply_cached_mesh_lightmap_override(&mut entity_camera_uniform, entity_light);
    let entity_camera_relative_position = camera_relative_entity_position_d_like_java(
        entity_position_d,
        ctx.entity_render_camera_origin_d,
    );
    let living_model_y_offset = living_entity_model_origin_y_offset_like_java(&entity.kind);
    let trans = glam::Mat4::from_translation(
        entity_camera_relative_position + glam::Vec3::Y * living_model_y_offset,
    );
    let type_key = remote_entity_type_key(&entity.kind);
    let rot = if type_key == "drowned" {
        let swim_amount = entity.render_swim_amount(ctx.partial_tick);
        if swim_amount > 0.0 {
            let x_rot = entity.render_pitch(ctx.partial_tick).to_degrees();
            let target_rot_x = -10.0 - x_rot;
            let x_angle = swim_amount * target_rot_x;
            glam::Mat4::from_rotation_y(-entity.render_body_yaw(ctx.partial_tick))
                * glam::Mat4::from_rotation_x(-x_angle.to_radians())
        } else {
            glam::Mat4::from_rotation_y(-entity.render_body_yaw(ctx.partial_tick))
        }
    } else {
        glam::Mat4::from_rotation_y(-entity.render_body_yaw(ctx.partial_tick))
    };
    let mut scale_mat = glam::Mat4::IDENTITY;
    if type_key == "slime" || type_key == "magma_cube" {
        let size = entity.mob_size as f32;
        let squish = entity.prev_squish + (entity.squish - entity.prev_squish) * ctx.partial_tick;
        let ss = squish / (size * 0.5 + 1.0);
        let w = 1.0 / (ss + 1.0);
        scale_mat = glam::Mat4::from_scale(glam::Vec3::new(w * size, (1.0 / w) * size, w * size));
        if type_key == "slime" {
            scale_mat = glam::Mat4::from_scale(glam::Vec3::splat(0.999))
                * glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.001, 0.0))
                * scale_mat;
        }
    }
    let entity_model_matrix = trans * rot * scale_mat;
    entity_camera_uniform.view_proj =
        (ctx.entity_view_proj * entity_model_matrix).to_cols_array_2d();
    rebase_model_space_fog_origin(
        &mut entity_camera_uniform,
        glam::Vec3::ZERO,
        entity_model_matrix.transform_point3(glam::Vec3::ZERO),
    );
    Some(PreparedFallbackEntity::RemoteEntity {
        #[cfg(feature = "shaderpacks")]
        entity_id,
        #[cfg(feature = "shaderpacks")]
        auxiliary_cull,
        #[cfg(feature = "shaderpacks")]
        model_matrix: entity_model_matrix,
        main_visible,
        texture_path,
        use_entity_cutout_pipeline,
        dynamic_mesh,
        wool_mesh,
        armor_meshes,
        camera_uniform: entity_camera_uniform,
    })
}
