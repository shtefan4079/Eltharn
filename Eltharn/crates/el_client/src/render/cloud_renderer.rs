use crate::render::sky_renderer::SkyCameraUniform;
use crate::resources::manager::ClientResourceManager;
use el_core::ResourceLocation;
use image::GenericImageView;
use wgpu::util::DeviceExt;
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RelativeCameraPos {
    AboveClouds,
    InsideClouds,
    BelowClouds,
}
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Direction {
    Down = 0,
    Up = 1,
    North = 2,
    South = 3,
    West = 4,
    East = 5,
}
impl Direction {
    fn get_3d_data_value(self) -> u32 {
        self as u32
    }
}
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CloudUniforms {
    color: [f32; 4],
    offset: [f32; 3],
    cloud_fog_end: f32,
    cell_size: [f32; 3],
    _pad2: f32,
}
pub struct CloudRenderer {
    cells: Vec<u64>,
    width: u32,
    height: u32,
    quad_count: u32,
    faces_buffer: Option<wgpu::Buffer>,
    static_index_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    flat_clouds_pipeline: wgpu::RenderPipeline,
    clouds_pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: Option<wgpu::BindGroup>,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    prev_cell_x: i32,
    prev_cell_z: i32,
    prev_relative_camera_pos: RelativeCameraPos,
    needs_rebuild: bool,
    was_fancy: bool,
}
impl CloudRenderer {
    #[cfg(target_arch = "wasm32")]
    pub fn try_new(
        _device: &wgpu::Device,
        _camera_bind_group_layout: &wgpu::BindGroupLayout,
        _texture_bind_group_layout: &wgpu::BindGroupLayout,
        _color_format: wgpu::TextureFormat,
        _depth_format: wgpu::TextureFormat,
        _resource_manager: &ClientResourceManager,
    ) -> Option<Self> {
        log::warn!(
            "CloudRenderer: storage buffers in vertex shader are not supported on WASM/WebGL2; disabling 3D cloud rendering"
        );
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_new(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        color_format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
        resource_manager: &ClientResourceManager,
    ) -> Option<Self> {
        let limits = device.limits();
        if limits.max_storage_buffers_per_shader_stage == 0 {
            log::warn!(
                "CloudRenderer: storage buffers in vertex shader are not supported on this adapter (max_storage_buffers_per_shader_stage=0); disabling 3D cloud rendering"
            );
            return None;
        }
        let (cells, width, height) = Self::load_cloud_cells(resource_manager);
        let mut indices = Vec::with_capacity(393216);
        for i in 0..65536 {
            let base = i * 4;
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let static_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cloud Static Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cloud Uniform Buffer"),
            size: std::mem::size_of::<CloudUniforms>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Cloud Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Cloud Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("cloud_shader.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cloud Pipeline Layout"),
            bind_group_layouts: &[
                camera_bind_group_layout,
                texture_bind_group_layout,
                &bind_group_layout,
            ],
            push_constant_ranges: &[],
        });
        let flat_clouds_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Flat Clouds Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let clouds_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Clouds Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: color_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: depth_format,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::GreaterEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Cloud Camera Buffer"),
            size: std::mem::size_of::<SkyCameraUniform>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cloud Camera Bind Group"),
            layout: camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        Some(Self {
            cells,
            width,
            height,
            quad_count: 0,
            faces_buffer: None,
            static_index_buffer,
            uniform_buffer,
            flat_clouds_pipeline,
            clouds_pipeline,
            bind_group_layout,
            bind_group: None,
            camera_buffer,
            camera_bind_group,
            prev_cell_x: i32::MIN,
            prev_cell_z: i32::MIN,
            prev_relative_camera_pos: RelativeCameraPos::InsideClouds,
            needs_rebuild: true,
            was_fancy: false,
        })
    }
    fn load_cloud_cells(resource_manager: &ClientResourceManager) -> (Vec<u64>, u32, u32) {
        let mut cells = Vec::new();
        let mut width = 0;
        let mut height = 0;
        let res_loc = ResourceLocation::new("minecraft", "environment/clouds");
        if let Ok(img) = resource_manager.read_image(&res_loc) {
            width = img.width();
            height = img.height();
            cells.resize((width * height) as usize, 0);
            for y in 0..height {
                for x in 0..width {
                    let pixel = img.get_pixel(x, y);
                    if pixel[3] < 10 {
                        continue;
                    }
                    let north = img.get_pixel(x, if y == 0 { height - 1 } else { y - 1 })[3] < 10;
                    let east = img.get_pixel(if x + 1 == width { 0 } else { x + 1 }, y)[3] < 10;
                    let south = img.get_pixel(x, if y + 1 == height { 0 } else { y + 1 })[3] < 10;
                    let west = img.get_pixel(if x == 0 { width - 1 } else { x - 1 }, y)[3] < 10;
                    let color = u32::from_be_bytes(pixel.0);
                    let mut packed = (color as u64) << 4;
                    if north {
                        packed |= 1 << 3;
                    }
                    if east {
                        packed |= 1 << 2;
                    }
                    if south {
                        packed |= 1 << 1;
                    }
                    if west {
                        packed |= 1;
                    }
                    cells[(x + y * width) as usize] = packed;
                }
            }
        }
        (cells, width, height)
    }
    pub fn reload_resource_texture(&mut self, resource_manager: &ClientResourceManager) {
        let (cells, width, height) = Self::load_cloud_cells(resource_manager);
        self.cells = cells;
        self.width = width;
        self.height = height;
        self.quad_count = 0;
        self.faces_buffer = None;
        self.bind_group = None;
        self.prev_cell_x = i32::MIN;
        self.prev_cell_z = i32::MIN;
        self.needs_rebuild = true;
    }
    fn is_north_empty(data: u64) -> bool {
        (data >> 3) & 1 != 0
    }
    fn is_east_empty(data: u64) -> bool {
        (data >> 2) & 1 != 0
    }
    fn is_south_empty(data: u64) -> bool {
        (data >> 1) & 1 != 0
    }
    fn is_west_empty(data: u64) -> bool {
        (data >> 0) & 1 != 0
    }
    pub fn draw_pass<'a>(
        &'a mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_pass: &mut wgpu::RenderPass<'a>,
        atlas_bind_group: &'a wgpu::BindGroup,
        translation_free_view_proj: glam::Mat4,
        camera_pos: glam::Vec3,
        game_time: i64,
        partial_tick: f32,
        fancy_clouds: bool,
        cloud_color: [f32; 4],
        bottom_y: f32,
        cloud_range: i32,
        cloud_fog_end: f32,
    ) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let cloud_offset_period = self.width as i64 * 400;
        let cloud_offset = game_time.rem_euclid(cloud_offset_period) as f32 + partial_tick;
        let mut cloud_x = camera_pos.x + cloud_offset * 0.03;
        let mut cloud_z = camera_pos.z + 3.96;
        let tex_width_blocks = (self.width * 12) as f32;
        let tex_height_blocks = (self.height * 12) as f32;
        cloud_x -= (cloud_x / tex_width_blocks).floor() * tex_width_blocks;
        cloud_z -= (cloud_z / tex_height_blocks).floor() * tex_height_blocks;
        let cell_x = (cloud_x / 12.0).floor() as i32;
        let cell_z = (cloud_z / 12.0).floor() as i32;
        let x_in_cell = cloud_x - cell_x as f32 * 12.0;
        let z_in_cell = cloud_z - cell_z as f32 * 12.0;
        let relative_bottom_y = bottom_y - camera_pos.y;
        let relative_top_y = relative_bottom_y + 4.0;
        let relative_camera_pos = if relative_top_y < 0.0 {
            RelativeCameraPos::AboveClouds
        } else if relative_bottom_y > 0.0 {
            RelativeCameraPos::BelowClouds
        } else {
            RelativeCameraPos::InsideClouds
        };
        let radius_blocks = cloud_range * 16;
        let radius_cells = (radius_blocks as f32 / 12.0).ceil() as i32;
        if self.needs_rebuild
            || cell_x != self.prev_cell_x
            || cell_z != self.prev_cell_z
            || relative_camera_pos != self.prev_relative_camera_pos
            || fancy_clouds != self.was_fancy
        {
            let mut face_data = Vec::new();
            self.build_mesh(
                relative_camera_pos,
                &mut face_data,
                cell_x,
                cell_z,
                fancy_clouds,
                radius_cells,
            );
            self.quad_count = face_data.len() as u32;
            if self.quad_count > 0 {
                if self.quad_count > 65536 {
                    self.quad_count = 65536;
                    face_data.truncate(65536);
                }
                let faces_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Cloud Faces Storage Buffer"),
                    contents: bytemuck::cast_slice(&face_data),
                    usage: wgpu::BufferUsages::STORAGE,
                });
                let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Cloud Bind Group"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: faces_buf.as_entire_binding(),
                        },
                    ],
                });
                self.faces_buffer = Some(faces_buf);
                self.bind_group = Some(bind_group);
            } else {
                self.faces_buffer = None;
                self.bind_group = None;
            }
            self.needs_rebuild = false;
            self.prev_cell_x = cell_x;
            self.prev_cell_z = cell_z;
            self.prev_relative_camera_pos = relative_camera_pos;
            self.was_fancy = fancy_clouds;
        }
        if self.quad_count == 0 || self.bind_group.is_none() {
            return;
        }
        let camera_uniform = SkyCameraUniform {
            view_proj: translation_free_view_proj.to_cols_array_2d(),
            lightmap_factors: [1.0, 1.0, 0.0, 0.0],
        };
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[camera_uniform]),
        );
        let uniforms = CloudUniforms {
            color: cloud_color,
            offset: [-x_in_cell, relative_bottom_y, -z_in_cell],
            cloud_fog_end,
            cell_size: [12.0, 4.0, 12.0],
            _pad2: 0.0,
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        let pipeline = if fancy_clouds {
            &self.clouds_pipeline
        } else {
            &self.flat_clouds_pipeline
        };
        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        render_pass.set_bind_group(1, atlas_bind_group, &[]);
        render_pass.set_bind_group(2, self.bind_group.as_ref().unwrap(), &[]);
        render_pass.set_index_buffer(
            self.static_index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        render_pass.draw_indexed(0..self.quad_count * 6, 0, 0..1);
    }
    fn build_mesh(
        &self,
        relative_pos: RelativeCameraPos,
        face_data: &mut Vec<u32>,
        center_cell_x: i32,
        center_cell_z: i32,
        extrude: bool,
        radius_cells: i32,
    ) {
        let cells = &self.cells;
        let width = self.width as i32;
        let height = self.height as i32;
        for ring in 0..=2 * radius_cells {
            for relative_cell_x in -ring..=ring {
                let relative_cell_z = ring - relative_cell_x.abs();
                if relative_cell_z >= 0
                    && relative_cell_z <= radius_cells
                    && relative_cell_x * relative_cell_x + relative_cell_z * relative_cell_z
                        <= radius_cells * radius_cells
                {
                    if relative_cell_z != 0 {
                        self.try_build_cell(
                            relative_pos,
                            face_data,
                            center_cell_x,
                            center_cell_z,
                            extrude,
                            relative_cell_x,
                            width,
                            -relative_cell_z,
                            height,
                            cells,
                        );
                    }
                    self.try_build_cell(
                        relative_pos,
                        face_data,
                        center_cell_x,
                        center_cell_z,
                        extrude,
                        relative_cell_x,
                        width,
                        relative_cell_z,
                        height,
                        cells,
                    );
                }
            }
        }
    }
    fn try_build_cell(
        &self,
        relative_pos: RelativeCameraPos,
        face_data: &mut Vec<u32>,
        cell_x: i32,
        cell_z: i32,
        extrude: bool,
        relative_cell_x: i32,
        texture_width: i32,
        relative_cell_z: i32,
        texture_height: i32,
        cells: &[u64],
    ) {
        let index_x = (cell_x + relative_cell_x).rem_euclid(texture_width);
        let index_y = (cell_z + relative_cell_z).rem_euclid(texture_height);
        let cell_data = cells[(index_x + index_y * texture_width) as usize];
        if cell_data != 0 {
            if extrude {
                self.build_extruded_cell(
                    relative_pos,
                    face_data,
                    relative_cell_x,
                    relative_cell_z,
                    cell_data,
                );
            } else {
                self.build_flat_cell(face_data, relative_cell_x, relative_cell_z);
            }
        }
    }
    fn build_flat_cell(&self, face_data: &mut Vec<u32>, x: i32, z: i32) {
        self.encode_face(face_data, x, z, Direction::Down, 32);
    }
    fn build_extruded_cell(
        &self,
        relative_pos: RelativeCameraPos,
        face_data: &mut Vec<u32>,
        x: i32,
        z: i32,
        cell_data: u64,
    ) {
        if relative_pos != RelativeCameraPos::BelowClouds {
            self.encode_face(face_data, x, z, Direction::Up, 0);
        }
        if relative_pos != RelativeCameraPos::AboveClouds {
            self.encode_face(face_data, x, z, Direction::Down, 0);
        }
        if Self::is_north_empty(cell_data) && z > 0 {
            self.encode_face(face_data, x, z, Direction::North, 0);
        }
        if Self::is_south_empty(cell_data) && z < 0 {
            self.encode_face(face_data, x, z, Direction::South, 0);
        }
        if Self::is_west_empty(cell_data) && x > 0 {
            self.encode_face(face_data, x, z, Direction::West, 0);
        }
        if Self::is_east_empty(cell_data) && x < 0 {
            self.encode_face(face_data, x, z, Direction::East, 0);
        }
        let add_interior_faces = x.abs() <= 1 && z.abs() <= 1;
        if add_interior_faces {
            for direction in &[
                Direction::Down,
                Direction::Up,
                Direction::North,
                Direction::South,
                Direction::West,
                Direction::East,
            ] {
                self.encode_face(face_data, x, z, *direction, 16);
            }
        }
    }
    fn encode_face(
        &self,
        face_data: &mut Vec<u32>,
        x: i32,
        z: i32,
        direction: Direction,
        flags: u32,
    ) {
        let mut dir_and_flags = direction.get_3d_data_value() | flags;
        dir_and_flags |= ((x & 1) as u32) << 7;
        dir_and_flags |= ((z & 1) as u32) << 6;
        let cell_x = (x >> 1) as u8;
        let cell_z = (z >> 1) as u8;
        let packed = (cell_x as u32) | ((cell_z as u32) << 8) | (dir_and_flags << 16);
        face_data.push(packed);
    }
}
