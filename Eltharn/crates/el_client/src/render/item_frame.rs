use super::*;
pub(super) fn frame_pose(spawn_data: i32) -> glam::Mat4 {
    let (step, x, y) = match (spawn_data % 6).unsigned_abs() {
        0 => (-glam::Vec3::Y, 90.0_f32, 180.0_f32),
        1 => (glam::Vec3::Y, -90.0, 180.0),
        2 => (-glam::Vec3::Z, 0.0, 0.0),
        3 => (glam::Vec3::Z, 0.0, 180.0),
        4 => (-glam::Vec3::X, 0.0, 90.0),
        _ => (glam::Vec3::X, 0.0, -90.0),
    };
    glam::Mat4::from_translation(step * 0.46875)
        * glam::Mat4::from_rotation_x(x.to_radians())
        * glam::Mat4::from_rotation_y(y.to_radians())
}
pub(super) fn item_pose(spawn_data: i32, rotation: i32, invisible: bool) -> glam::Mat4 {
    frame_pose(spawn_data)
        * glam::Mat4::from_translation(glam::Vec3::Z * if invisible { 0.5 } else { 0.4375 })
        * glam::Mat4::from_rotation_z((rotation as f32 * 45.0).to_radians())
        * glam::Mat4::from_scale(glam::Vec3::splat(0.5))
}
fn frame_model_id(glow: bool, map: bool) -> ResourceLocation {
    ResourceLocation::new(
        "minecraft",
        match (glow, map) {
            (false, false) => "block/item_frame",
            (false, true) => "block/item_frame_map",
            (true, false) => "block/glow_item_frame",
            (true, true) => "block/glow_item_frame_map",
        },
    )
}
fn packed_frame_light(block: u8, sky: u8, burning: bool, glow: bool) -> [f32; 2] {
    let block = if burning { 15 } else { block };
    packed_lightmap_factors(if glow { block.max(5) } else { block }, sky)
}
pub(super) fn prepare(
    ctx: &FallbackEntityPrepareContext<'_>,
    entity: &RemoteEntityRenderState,
) -> Option<PreparedFallbackEntity> {
    if !remote_entity_should_render_at_distance_like_java(
        entity,
        ctx.camera.position,
        ctx.entity_view_scale,
        ctx.partial_tick,
    ) {
        return None;
    }
    let position = entity.render_position(ctx.partial_tick);
    if !ctx
        .terrain_frustum
        .contains_aabb(position - glam::Vec3::ONE, position + glam::Vec3::ONE)
    {
        return None;
    }
    let glow = remote_entity_type_key_eq(&entity.kind, "glow_item_frame");
    let invisible = entity.shared_flag(5);
    let stack = entity.item_stack.as_ref().filter(|stack| !stack.is_empty());
    let has_map = stack
        .and_then(crate::render::map::map_id)
        .is_some_and(|id| ctx.maps.contains_key(&id));
    let light = ctx.level.map_or([15.0 / 16.0; 2], |level| {
        let pos = BlockPos::new(
            position.x.floor() as i32,
            position.y.floor() as i32,
            position.z.floor() as i32,
        );
        packed_frame_light(
            level.brightness(false, pos),
            level.brightness(true, pos),
            entity.shared_flag(0),
            glow,
        )
    });
    let relative = camera_relative_entity_position_d_like_java(
        entity.render_position_d(ctx.partial_tick),
        ctx.entity_render_camera_origin_d,
    );
    let translation = glam::Mat4::from_translation(relative);
    let uniform = |light: [f32; 2]| {
        let mut uniform = ctx.camera_uniform;
        uniform.overlay_color = [1.0; 4];
        apply_cached_mesh_lightmap_override(&mut uniform, light);
        uniform.view_proj = ctx.entity_view_proj.to_cols_array_2d();
        uniform.fog_camera_position = [0.0; 4];
        uniform
    };
    let frame = if invisible {
        None
    } else {
        let model_id = frame_model_id(glow, has_map);
        BlockModel::load(ctx.resource_manager, &model_id)
            .ok()
            .and_then(|model| {
                let baked = BakedModel::bake(&model, ctx.atlas, 0, 0, false);
                let mut vertices = Vec::new();
                let mut indices = Vec::new();
                for quad in baked.quads_for_seed(42) {
                    push_transformed_quad(
                        &mut vertices,
                        &mut indices,
                        &glam::Mat4::IDENTITY,
                        quad.vertex_positions,
                        quad.vertex_uvs,
                        direction_normal(quad.direction),
                        [1.0; 4],
                    );
                }
                if indices.is_empty() {
                    return None;
                }
                let model = glam::Mat4::from_scale(glam::Vec3::splat(1.0 + 1.0 / 4096.0))
                    * translation
                    * frame_pose(entity.spawn_data)
                    * glam::Mat4::from_translation(glam::Vec3::splat(-0.5));
                transform_mesh(&mut vertices, model);
                Some((vertices, indices, uniform(light)))
            })
    };
    let item = stack.filter(|_| !has_map).and_then(|stack| {
        let is_head = stack.item.location.path == "player_head";
        let (mut vertices, indices) = if is_head {
            generate_player_head_item_mesh(
                &stack.item.location,
                ctx.resource_manager,
                crate::render::model::player::player_model::HandSide::Right,
                HeldItemDisplay::Fixed,
            )
        } else {
            generate_held_item_mesh_for_display(
                &stack.item.location,
                ctx.atlas,
                ctx.resource_manager,
                crate::render::model::player::player_model::HandSide::Right,
                HeldItemDisplay::Fixed,
                ctx.block_colors,
                Some(stack),
            )
        };
        if indices.is_empty() {
            return None;
        }
        let model =
            translation * item_pose(entity.spawn_data, entity.item_frame_rotation, invisible);
        let item_light = if glow { [240.0 / 256.0; 2] } else { light };
        transform_mesh(&mut vertices, model);
        let texture = is_head.then(|| {
            ctx.frame_head_texture_keys
                .get(&entity.id)
                .cloned()
                .unwrap_or_else(|| {
                    default_player_skin_texture_path(
                        player_head_profile_uuid_from_item(stack).unwrap_or_else(uuid::Uuid::nil),
                    )
                    .to_string()
                })
        });
        Some((
            vertices,
            indices,
            uniform(item_light),
            item_has_foil(stack) && !is_head,
            texture,
        ))
    });
    (frame.is_some() || item.is_some()).then_some(PreparedFallbackEntity::ItemFrame { frame, item })
}
fn transform_mesh(vertices: &mut [Vertex], model: glam::Mat4) {
    for vertex in vertices {
        vertex.position = model
            .transform_point3(glam::Vec3::from_array(vertex.position))
            .to_array();
        vertex.normal = model
            .transform_vector3(glam::Vec3::from_array(vertex.normal))
            .normalize_or_zero()
            .to_array();
    }
}
pub(super) fn draw_mesh<'a>(
    pass: &mut CountedRenderPass<'a, '_>,
    binding: crate::render::frame_mesh_arena::FrameMeshBinding<'a>,
    pipeline: &'a wgpu::RenderPipeline,
    texture: &'a wgpu::BindGroup,
    camera: &'a wgpu::BindGroup,
) {
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, camera, &[]);
    pass.set_bind_group(1, texture, &[]);
    pass.set_vertex_buffer(0, binding.vertex_buffer().slice(binding.vertex_range()));
    pass.set_index_buffer(
        binding.index_buffer().slice(binding.index_range()),
        wgpu::IndexFormat::Uint32,
    );
    pass.draw_indexed(0..binding.index_count(), 0, 0..1);
}
pub(super) fn missing_texture_binding(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::BindGroup {
    let image = crate::render::texture::generate_missing_texture_image();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("TextureManager missing entity image"),
        size: wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &image,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(64),
            rows_per_image: Some(16),
        },
        wgpu::Extent3d {
            width: 16,
            height: 16,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Missing entity image binding"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    })
}
pub(super) fn frame_pipeline(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    item: bool,
) -> wgpu::RenderPipeline {
    create_pipeline(
        device,
        camera_layout,
        texture_layout,
        format,
        if item {
            "fs_item_frame"
        } else {
            "fs_entity_solid"
        },
        item,
        Some(wgpu::Face::Back),
    )
}
pub(super) fn head_pipeline(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    create_pipeline(
        device,
        camera_layout,
        texture_layout,
        format,
        "fs_translucent",
        true,
        None,
    )
}
fn create_pipeline(
    device: &wgpu::Device,
    camera_layout: &wgpu::BindGroupLayout,
    texture_layout: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    fragment: &str,
    blend: bool,
    cull_mode: Option<wgpu::Face>,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Item frame entity solid shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Item frame entity solid layout"),
        bind_group_layouts: &[camera_layout, texture_layout],
        push_constant_ranges: &[],
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Item frame entity solid offset forward"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![
                    0 => Float32x3, 1 => Float32x2, 2 => Float32x3, 3 =>
                    Float32x4, 4 => Float32x2
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: fragment,
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: blend.then_some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::GreaterEqual,
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview: None,
    })
}
