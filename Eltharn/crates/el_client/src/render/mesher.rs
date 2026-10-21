#![allow(dead_code)]
use crate::render::baked_model::{BakedModel, Vertex};
use crate::render::block_colors::{Biome, BlockAndTintGetter, ClientBlockColors};
use crate::render::blockstate::effective_properties;
use crate::world::client_level::ClientLevel;
use el_core::block_behaviour::{
    java_26_2_vanilla_block_fluid_support_properties, java_26_2_vanilla_block_lighting_properties,
};
use el_core::block_state::BlockState;
use el_math::block_pos::BlockPos;
use el_math::{Direction, VoxelShape};
use el_world::chunk::Chunk;
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
pub(crate) struct MesherAtomicStats {
    pub(crate) completed: AtomicU64,
    pub(crate) total_us: AtomicU64,
    pub(crate) max_us: AtomicU64,
}
pub(crate) static MESHER_STATS: MesherAtomicStats = MesherAtomicStats {
    completed: AtomicU64::new(0),
    total_us: AtomicU64::new(0),
    max_us: AtomicU64::new(0),
};
#[inline(always)]
pub(crate) fn record_mesh_done(elapsed_us: u64) {
    MESHER_STATS.completed.fetch_add(1, Ordering::Relaxed);
    MESHER_STATS
        .total_us
        .fetch_add(elapsed_us, Ordering::Relaxed);
    let mut current_max = MESHER_STATS.max_us.load(Ordering::Relaxed);
    while elapsed_us > current_max {
        match MESHER_STATS.max_us.compare_exchange_weak(
            current_max,
            elapsed_us,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(actual) => current_max = actual,
        }
    }
}
struct MeshTimerGuard(Option<std::time::Instant>);
impl MeshTimerGuard {
    #[inline]
    fn new() -> Self {
        Self(mesher_stats_enabled().then(std::time::Instant::now))
    }
}
impl Drop for MeshTimerGuard {
    fn drop(&mut self) {
        if let Some(started_at) = self.0 {
            crate::render::mesher::record_mesh_done(started_at.elapsed().as_micros() as u64);
        }
    }
}
static DISABLE_FACE_CULLING: OnceLock<bool> = OnceLock::new();
static MESHER_STATS_ENABLED: OnceLock<bool> = OnceLock::new();
#[inline]
fn mesher_stats_enabled() -> bool {
    *MESHER_STATS_ENABLED.get_or_init(|| {
        std::env::var("ELTHARN_RENDER_PROFILE")
            .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
    })
}
const SECTION_BRIGHTNESS_CACHE_SIZE: usize = 100;
thread_local! {
    static SECTION_BRIGHTNESS_SCRATCH : RefCell < Option < SectionBrightnessCache >> =
    RefCell::new(Some(SectionBrightnessCache::new()));
}
struct SectionBrightnessCache {
    samples: HashMap<i64, [u8; 2]>,
    insertion_order: VecDeque<i64>,
}
impl SectionBrightnessCache {
    fn new() -> Self {
        Self {
            samples: HashMap::with_capacity(SECTION_BRIGHTNESS_CACHE_SIZE),
            insertion_order: VecDeque::with_capacity(SECTION_BRIGHTNESS_CACHE_SIZE),
        }
    }
    fn get_or_insert_with(&mut self, pos: BlockPos, sample: impl FnOnce() -> [u8; 2]) -> [u8; 2] {
        let key = pos.as_long();
        if let Some(&cached) = self.samples.get(&key) {
            return cached;
        }
        let sampled = sample();
        if self.samples.len() == SECTION_BRIGHTNESS_CACHE_SIZE
            && let Some(oldest) = self.insertion_order.pop_front()
        {
            self.samples.remove(&oldest);
        }
        self.samples.insert(key, sampled);
        self.insertion_order.push_back(key);
        sampled
    }
    fn clear(&mut self) {
        self.samples.clear();
        self.insertion_order.clear();
    }
}
impl Default for SectionBrightnessCache {
    fn default() -> Self {
        Self {
            samples: HashMap::new(),
            insertion_order: VecDeque::new(),
        }
    }
}
struct SectionBrightnessCacheLease {
    cache: RefCell<SectionBrightnessCache>,
}
impl SectionBrightnessCacheLease {
    fn acquire() -> Self {
        let cache = SECTION_BRIGHTNESS_SCRATCH
            .with(|scratch| scratch.borrow_mut().take())
            .unwrap_or_else(SectionBrightnessCache::new);
        Self {
            cache: RefCell::new(cache),
        }
    }
    fn cache(&self) -> &RefCell<SectionBrightnessCache> {
        &self.cache
    }
}
impl Drop for SectionBrightnessCacheLease {
    fn drop(&mut self) {
        let mut cache = self.cache.take();
        cache.clear();
        SECTION_BRIGHTNESS_SCRATCH.with(|scratch| {
            let mut scratch = scratch.borrow_mut();
            if scratch.is_none() {
                *scratch = Some(cache);
            }
        });
    }
}
pub struct ChunkMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub translucent_indices: Vec<u32>,
    pub visibility: crate::render::chunk::VisibilitySet,
}
#[derive(Default)]
pub struct MovingBlockMesh {
    pub vertices: Vec<Vertex>,
    pub solid_indices: Vec<u32>,
    pub cutout_indices: Vec<u32>,
    pub translucent_indices: Vec<u32>,
}
impl MovingBlockMesh {
    pub fn is_empty(&self) -> bool {
        self.solid_indices.is_empty()
            && self.cutout_indices.is_empty()
            && self.translucent_indices.is_empty()
    }
}
pub struct ChunkMesher;
fn is_java_invisible_moving_piston(state: &BlockState) -> bool {
    state.block.namespace == "minecraft" && state.block.path == "moving_piston"
}
impl ChunkMesher {
    pub fn block_state_model_key(block_state: &BlockState) -> String {
        let properties = effective_properties(block_state);
        if properties.is_empty() {
            return block_state.block.to_string();
        }
        let mut sorted: Vec<_> = properties.iter().collect();
        sorted.sort_by(|(left, _), (right, _)| left.cmp(right));
        let property_string = sorted
            .into_iter()
            .map(|(name, value)| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}[{}]", block_state.block, property_string)
    }
    pub fn mesh_chunk_with_level(
        chunk: &Chunk,
        cx: i32,
        cz: i32,
        model_cache: &HashMap<String, BakedModel>,
        block_colors: &ClientBlockColors,
        level: Option<&ClientLevel>,
        ambient_occlusion_enabled: bool,
    ) -> ChunkMesh {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut translucent_indices = Vec::new();
        let mut index_offset = 0;
        let mut visibility = crate::render::chunk::VisibilitySet::default();
        visibility.set_all(true);
        let mut baked_quad_scratch = Vec::new();
        let mut models_by_state: HashMap<usize, Option<&BakedModel>> = HashMap::new();
        let mut block_names_by_state: HashMap<usize, String> = HashMap::new();
        let mut face_culling_by_state_pair: HashMap<(usize, usize, Direction), bool> =
            HashMap::new();
        let neighbor_chunks = level
            .map(|level| {
                [
                    (cx - 1, cz),
                    (cx + 1, cz),
                    (cx, cz - 1),
                    (cx, cz + 1),
                    (cx - 1, cz - 1),
                    (cx - 1, cz + 1),
                    (cx + 1, cz - 1),
                    (cx + 1, cz + 1),
                ]
                .into_iter()
                .filter_map(|(neighbor_x, neighbor_z)| {
                    level
                        .chunks
                        .get(&crate::world::client_level::ChunkPos::new(
                            neighbor_x, neighbor_z,
                        ))
                        .map(|chunk| chunk.as_ref())
                })
                .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let neighbor_grid = level.map(|_| NeighborGrid::new(chunk, &neighbor_chunks));
        for section in &chunk.sections {
            let section_y_base = section.y_index as i32 * 16;
            for y in 0..16 {
                for z in 0..16 {
                    for x in 0..16 {
                        let block_state = section.get_block_state_ref(x, y, z);
                        let fluid_state = block_state.get_fluid_state();
                        if (block_state.block.path == "air" || block_state.block.path.is_empty())
                            && fluid_state.is_empty()
                        {
                            continue;
                        }
                        let block_path = block_state.block.path.as_str();
                        if is_java_invisible_moving_piston(block_state) {
                            continue;
                        }
                        if !fluid_state.is_empty() {
                            let world_x = cx * 16 + x as i32;
                            let world_y = section_y_base + y as i32;
                            let world_z = cz * 16 + z as i32;
                            let block_pos = BlockPos::new(world_x, world_y, world_z);
                            let view = LevelBlockView { chunk, level };
                            FluidRenderer::tesselate(
                                &view,
                                block_pos,
                                block_state,
                                block_colors,
                                level.map(|level| level as &dyn BlockAndTintGetter),
                                &mut vertices,
                                &mut indices,
                                &mut translucent_indices,
                                &mut index_offset,
                            );
                            if block_path == "water" || block_path == "lava" {
                                continue;
                            }
                        }
                        let state_cache_key = block_state as *const BlockState as usize;
                        let baked_model =
                            *models_by_state.entry(state_cache_key).or_insert_with(|| {
                                Self::baked_model_for_state(block_state, model_cache)
                            });
                        if let Some(baked_model) = baked_model {
                            let world_x = cx * 16 + x as i32;
                            let world_y = section_y_base + y as i32;
                            let world_z = cz * 16 + z as i32;
                            let block_pos = BlockPos::new(world_x, world_y, world_z);
                            let block_name = block_names_by_state
                                .entry(state_cache_key)
                                .or_insert_with(|| block_state.block.to_string());
                            let model_offset =
                                el_core::block_behaviour::java_26_2_vanilla_block_model_offset(
                                    block_name,
                                    [world_x, world_y, world_z],
                                )
                                .unwrap_or([0.0, 0.0, 0.0]);
                            let seed = block_model_seed(block_state, block_pos);
                            baked_model.collect_quads_for_seed_into(seed, &mut baked_quad_scratch);
                            for quad in baked_quad_scratch.iter().copied() {
                                let view = LevelBlockView { chunk, level };
                                let (vertex_ao, vertex_light) = quad_vertex_lighting(
                                    &view,
                                    block_pos,
                                    block_state,
                                    quad,
                                    ambient_occlusion_enabled,
                                );
                                let quad_is_culled = if let Some(grid) = neighbor_grid {
                                    Self::is_quad_culled_with_grid_pointer_cached(
                                        grid,
                                        block_state,
                                        block_pos,
                                        quad,
                                        &mut face_culling_by_state_pair,
                                    )
                                } else {
                                    Self::is_quad_culled(chunk, level, block_state, block_pos, quad)
                                };
                                if quad_is_culled {
                                    continue;
                                }
                                let base_color = block_colors.tint_for_quad(
                                    block_state,
                                    quad,
                                    level.map(|level| level as &dyn BlockAndTintGetter),
                                    block_pos,
                                );
                                let normal = if !quad.shade {
                                    [0.0, 1.0, 0.0]
                                } else {
                                    match quad.direction {
                                        el_math::Direction::Down => [0.0, -1.0, 0.0],
                                        el_math::Direction::Up => [0.0, 1.0, 0.0],
                                        el_math::Direction::North => [0.0, 0.0, -1.0],
                                        el_math::Direction::South => [0.0, 0.0, 1.0],
                                        el_math::Direction::West => [-1.0, 0.0, 0.0],
                                        el_math::Direction::East => [1.0, 0.0, 0.0],
                                    }
                                };
                                for i in 0..4 {
                                    let pos = quad.vertex_positions[i];
                                    let uv = quad.vertex_uvs[i];
                                    let mut color = base_color;
                                    color[0] *= vertex_ao[i];
                                    color[1] *= vertex_ao[i];
                                    color[2] *= vertex_ao[i];
                                    vertices.push(Vertex {
                                        position: [
                                            pos.x + world_x as f32 + model_offset[0],
                                            pos.y + world_y as f32 + model_offset[1],
                                            pos.z + world_z as f32 + model_offset[2],
                                        ],
                                        tex_coords: uv,
                                        normal,
                                        color,
                                        light: vertex_light[i],
                                    });
                                }
                                let layer_indices =
                                    if Self::quad_should_use_translucent_layer(block_state, quad) {
                                        &mut translucent_indices
                                    } else {
                                        &mut indices
                                    };
                                layer_indices.extend_from_slice(&[
                                    index_offset,
                                    index_offset + 1,
                                    index_offset + 2,
                                    index_offset + 2,
                                    index_offset + 3,
                                    index_offset,
                                ]);
                                index_offset += 4;
                            }
                        }
                    }
                }
            }
        }
        ChunkMesh {
            vertices,
            indices,
            translucent_indices,
            visibility,
        }
    }
    pub fn mesh_chunk_section_with_neighbors(
        chunk: &Chunk,
        section_index: usize,
        neighbor_chunks: &[&Chunk],
        model_cache: &HashMap<String, BakedModel>,
        block_colors: &ClientBlockColors,
        biome_definitions: &HashMap<String, crate::render::block_colors::Biome>,
        biome_blend_radius: i32,
        biome_zoom_seed: i64,
        ambient_occlusion_enabled: bool,
    ) -> ChunkMesh {
        let _timer = MeshTimerGuard::new();
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut translucent_indices = Vec::new();
        let mut index_offset = 0;
        let mut vis_graph = crate::render::chunk::VisGraph::new();
        let mut full_occlusion_by_state = HashMap::new();
        let mut fluid_states_by_state = HashMap::new();
        let mut models_by_state: HashMap<usize, Option<&BakedModel>> = HashMap::new();
        let mut face_culling_by_state_pair = HashMap::new();
        let mut baked_quad_scratch = Vec::new();
        let mut block_id_str_by_state: HashMap<usize, String> = HashMap::new();
        let Some(section) = chunk.sections.get(section_index) else {
            return ChunkMesh {
                vertices,
                indices,
                translucent_indices,
                visibility: vis_graph.resolve(),
            };
        };
        if section.has_only_air() {
            return ChunkMesh {
                vertices,
                indices,
                translucent_indices,
                visibility: vis_graph.resolve(),
            };
        }
        let section_y_base = section.y_index * 16;
        let grid = NeighborGrid::new(chunk, neighbor_chunks);
        let uncached_view = NeighborSnapshotBlockView { grid };
        let brightness_cache = SectionBrightnessCacheLease::acquire();
        let view = CachedSectionBlockView {
            inner: &uncached_view,
            brightness_cache: brightness_cache.cache(),
        };
        let tint_view = SnapshotTintView {
            grid,
            block_colors,
            biome_definitions,
            biome_blend_radius,
            biome_zoom_seed,
        };
        for y in 0..16 {
            for z in 0..16 {
                for x in 0..16 {
                    let block_state = section.get_block_state_ref(x, y, z);
                    let fluid_state = if block_state.id == 0 {
                        block_state.get_fluid_state()
                    } else {
                        *fluid_states_by_state
                            .entry(block_state.id)
                            .or_insert_with(|| block_state.get_fluid_state())
                    };
                    if (block_state.block.path == "air" || block_state.block.path.is_empty())
                        && fluid_state.is_empty()
                    {
                        continue;
                    }
                    let block_path = block_state.block.path.as_str();
                    if is_java_invisible_moving_piston(block_state) {
                        continue;
                    }
                    let full_occlusion = if block_state.id == 0 {
                        Self::occlusion_shape_is_full_block(&block_state.get_occlusion_shape())
                    } else {
                        *full_occlusion_by_state
                            .entry(block_state.id)
                            .or_insert_with(|| {
                                Self::occlusion_shape_is_full_block(
                                    &block_state.get_occlusion_shape(),
                                )
                            })
                    };
                    if full_occlusion {
                        vis_graph.set_opaque(x, y, z);
                    }
                    if !fluid_state.is_empty() {
                        let world_x = chunk.x * 16 + x;
                        let world_y = section_y_base + y;
                        let world_z = chunk.z * 16 + z;
                        let block_pos = BlockPos::new(world_x, world_y, world_z);
                        FluidRenderer::tesselate(
                            &view,
                            block_pos,
                            block_state,
                            block_colors,
                            Some(&tint_view),
                            &mut vertices,
                            &mut indices,
                            &mut translucent_indices,
                            &mut index_offset,
                        );
                        if block_path == "water" || block_path == "lava" {
                            continue;
                        }
                    }
                    let baked_model = *models_by_state
                        .entry(block_state.id)
                        .or_insert_with(|| Self::baked_model_for_state(block_state, model_cache));
                    if let Some(baked_model) = baked_model {
                        let world_x = chunk.x * 16 + x;
                        let world_y = section_y_base + y;
                        let world_z = chunk.z * 16 + z;
                        let block_pos = BlockPos::new(world_x, world_y, world_z);
                        let block_id_str = block_id_str_by_state
                            .entry(block_state.id)
                            .or_insert_with(|| block_state.block.to_string());
                        let model_offset =
                            el_core::block_behaviour::java_26_2_vanilla_block_model_offset(
                                block_id_str,
                                [world_x, world_y, world_z],
                            )
                            .unwrap_or([0.0, 0.0, 0.0]);
                        let seed =
                            el_core::block_behaviour::java_26_2_vanilla_block_model_seed_with_id(
                                block_id_str,
                                block_state,
                                block_pos,
                            )
                            .unwrap_or_else(|| generic_java_block_model_seed(block_pos));
                        baked_model.collect_quads_for_seed_into(seed, &mut baked_quad_scratch);
                        for quad in baked_quad_scratch.iter().copied() {
                            if Self::is_quad_culled_with_grid_cached(
                                grid,
                                block_state,
                                block_pos,
                                quad,
                                &mut face_culling_by_state_pair,
                            ) {
                                continue;
                            }
                            let base_color = block_colors.tint_for_quad(
                                block_state,
                                quad,
                                Some(&tint_view),
                                block_pos,
                            );
                            let normal = if !quad.shade {
                                [0.0, 1.0, 0.0]
                            } else {
                                match quad.direction {
                                    el_math::Direction::Down => [0.0, -1.0, 0.0],
                                    el_math::Direction::Up => [0.0, 1.0, 0.0],
                                    el_math::Direction::North => [0.0, 0.0, -1.0],
                                    el_math::Direction::South => [0.0, 0.0, 1.0],
                                    el_math::Direction::West => [-1.0, 0.0, 0.0],
                                    el_math::Direction::East => [1.0, 0.0, 0.0],
                                }
                            };
                            let (vertex_ao, vertex_light) = quad_vertex_lighting(
                                &view,
                                block_pos,
                                block_state,
                                quad,
                                ambient_occlusion_enabled,
                            );
                            for i in 0..4 {
                                let pos = quad.vertex_positions[i];
                                let uv = quad.vertex_uvs[i];
                                let mut color = base_color;
                                color[0] *= vertex_ao[i];
                                color[1] *= vertex_ao[i];
                                color[2] *= vertex_ao[i];
                                vertices.push(Vertex {
                                    position: [
                                        pos.x + world_x as f32 + model_offset[0],
                                        pos.y + world_y as f32 + model_offset[1],
                                        pos.z + world_z as f32 + model_offset[2],
                                    ],
                                    tex_coords: uv,
                                    normal,
                                    color,
                                    light: vertex_light[i],
                                });
                            }
                            let layer_indices =
                                if Self::quad_should_use_translucent_layer(block_state, quad) {
                                    &mut translucent_indices
                                } else {
                                    &mut indices
                                };
                            layer_indices.extend_from_slice(&[
                                index_offset,
                                index_offset + 1,
                                index_offset + 2,
                                index_offset + 2,
                                index_offset + 3,
                                index_offset,
                            ]);
                            index_offset += 4;
                        }
                    }
                }
            }
        }
        ChunkMesh {
            vertices,
            indices,
            translucent_indices,
            visibility: vis_graph.resolve(),
        }
    }
    pub fn mesh_moving_block_like_java(
        block_state: &BlockState,
        block_pos: BlockPos,
        random_seed_pos: BlockPos,
        level: &ClientLevel,
        model_cache: &HashMap<String, BakedModel>,
        ambient_occlusion_enabled: bool,
        cutout_leaves: bool,
    ) -> Option<MovingBlockMesh> {
        let mut baked_quad_scratch = Vec::new();
        Self::mesh_moving_block_like_java_with_scratch(
            block_state,
            block_pos,
            random_seed_pos,
            level,
            model_cache,
            ambient_occlusion_enabled,
            cutout_leaves,
            &mut baked_quad_scratch,
        )
    }
    pub fn mesh_moving_block_like_java_with_scratch<'a>(
        block_state: &BlockState,
        block_pos: BlockPos,
        random_seed_pos: BlockPos,
        level: &ClientLevel,
        model_cache: &'a HashMap<String, BakedModel>,
        ambient_occlusion_enabled: bool,
        cutout_leaves: bool,
        baked_quad_scratch: &mut Vec<&'a crate::render::baked_model::BakedQuad>,
    ) -> Option<MovingBlockMesh> {
        let baked_model = Self::baked_model_for_state(block_state, model_cache)?;
        let moving_view = MovingBlockRenderView::new(level, block_pos, block_state);
        let model_offset = el_core::block_behaviour::java_26_2_vanilla_block_model_offset(
            &block_state.block.to_string(),
            [block_pos.x, block_pos.y, block_pos.z],
        )
        .unwrap_or([0.0, 0.0, 0.0]);
        let force_opaque = !cutout_leaves && is_java_26_2_leaves_block(block_state);
        let mut mesh = MovingBlockMesh::default();
        let mut index_offset = 0_u32;
        baked_model.collect_quads_for_seed_into(
            block_model_seed(block_state, random_seed_pos),
            baked_quad_scratch,
        );
        for quad in baked_quad_scratch.iter().copied() {
            let (vertex_ao, vertex_light) = quad_vertex_lighting(
                &moving_view,
                block_pos,
                block_state,
                quad,
                ambient_occlusion_enabled,
            );
            let base_color =
                level
                    .block_colors
                    .tint_for_quad(block_state, quad, Some(&moving_view), block_pos);
            let normal = terrain_cardinal_normal_for_baked_quad(quad);
            for vertex in 0..4 {
                let position = quad.vertex_positions[vertex];
                let mut color = base_color;
                color[0] *= vertex_ao[vertex];
                color[1] *= vertex_ao[vertex];
                color[2] *= vertex_ao[vertex];
                mesh.vertices.push(Vertex {
                    position: [
                        position.x + model_offset[0],
                        position.y + model_offset[1],
                        position.z + model_offset[2],
                    ],
                    tex_coords: quad.vertex_uvs[vertex],
                    normal,
                    color,
                    light: vertex_light[vertex],
                });
            }
            let indices = if force_opaque {
                &mut mesh.solid_indices
            } else {
                match quad.layer {
                    crate::render::chunk::ChunkSectionLayer::Solid => &mut mesh.solid_indices,
                    crate::render::chunk::ChunkSectionLayer::Cutout => &mut mesh.cutout_indices,
                    crate::render::chunk::ChunkSectionLayer::Translucent => {
                        &mut mesh.translucent_indices
                    }
                }
            };
            indices.extend_from_slice(&[
                index_offset,
                index_offset + 1,
                index_offset + 2,
                index_offset + 2,
                index_offset + 3,
                index_offset,
            ]);
            index_offset += 4;
        }
        Some(mesh)
    }
    pub fn mesh_block_model_instance_like_java(
        block_state: &BlockState,
        model_cache: &HashMap<String, BakedModel>,
        light: [f32; 2],
    ) -> Option<MovingBlockMesh> {
        let mut baked_quad_scratch = Vec::new();
        Self::mesh_block_model_instance_like_java_with_scratch(
            block_state,
            model_cache,
            light,
            &mut baked_quad_scratch,
        )
    }
    pub fn mesh_block_model_instance_like_java_with_scratch<'a>(
        block_state: &BlockState,
        model_cache: &'a HashMap<String, BakedModel>,
        light: [f32; 2],
        baked_quad_scratch: &mut Vec<&'a crate::render::baked_model::BakedQuad>,
    ) -> Option<MovingBlockMesh> {
        let baked_model = Self::baked_model_for_state(block_state, model_cache)?;
        let mut mesh = MovingBlockMesh::default();
        let mut index_offset = 0_u32;
        baked_model.collect_quads_for_seed_into(42, baked_quad_scratch);
        for quad in baked_quad_scratch.iter().copied() {
            let normal = terrain_cardinal_normal_for_baked_quad(quad);
            for vertex in 0..4 {
                mesh.vertices.push(Vertex {
                    position: quad.vertex_positions[vertex].to_array(),
                    tex_coords: quad.vertex_uvs[vertex],
                    normal,
                    color: [1.0; 4],
                    light,
                });
            }
            let indices = match quad.layer {
                crate::render::chunk::ChunkSectionLayer::Solid => &mut mesh.solid_indices,
                crate::render::chunk::ChunkSectionLayer::Cutout => &mut mesh.cutout_indices,
                crate::render::chunk::ChunkSectionLayer::Translucent => {
                    &mut mesh.translucent_indices
                }
            };
            indices.extend_from_slice(&[
                index_offset,
                index_offset + 1,
                index_offset + 2,
                index_offset + 2,
                index_offset + 3,
                index_offset,
            ]);
            index_offset += 4;
        }
        Some(mesh)
    }
    fn is_quad_culled(
        chunk: &Chunk,
        level: Option<&ClientLevel>,
        block_state: &BlockState,
        block_pos: BlockPos,
        quad: &crate::render::baked_model::BakedQuad,
    ) -> bool {
        if Self::debug_disable_face_culling() {
            return false;
        }
        let Some(cull_direction) = quad.cullface.as_deref().and_then(Direction::from_name) else {
            return false;
        };
        let (dx, dy, dz) = cull_direction.step();
        let neighbor_pos = BlockPos::new(block_pos.x + dx, block_pos.y + dy, block_pos.z + dz);
        let neighbor_state = if let Some(level) = level {
            level.get_block_state(neighbor_pos)
        } else {
            let Some(neighbor_state) = chunk.get_block_state(&neighbor_pos) else {
                return false;
            };
            neighbor_state
        };
        !Self::should_render_face(block_state, &neighbor_state, cull_direction)
    }
    fn is_quad_culled_with_grid_cached(
        grid: NeighborGrid,
        block_state: &BlockState,
        block_pos: BlockPos,
        quad: &crate::render::baked_model::BakedQuad,
        cache: &mut HashMap<(usize, usize, Direction), bool>,
    ) -> bool {
        if Self::debug_disable_face_culling() {
            return false;
        }
        let Some(cull_direction) = quad.cullface.as_deref().and_then(Direction::from_name) else {
            return false;
        };
        let (dx, dy, dz) = cull_direction.step();
        let neighbor_pos = BlockPos::new(block_pos.x + dx, block_pos.y + dy, block_pos.z + dz);
        let Some(neighbor_state) = grid.get_block_state_ref(&neighbor_pos) else {
            return false;
        };
        *cache
            .entry((block_state.id, neighbor_state.id, cull_direction))
            .or_insert_with(|| {
                !Self::should_render_face(block_state, &neighbor_state, cull_direction)
            })
    }
    fn is_quad_culled_with_grid_pointer_cached(
        grid: NeighborGrid,
        block_state: &BlockState,
        block_pos: BlockPos,
        quad: &crate::render::baked_model::BakedQuad,
        cache: &mut HashMap<(usize, usize, Direction), bool>,
    ) -> bool {
        if Self::debug_disable_face_culling() {
            return false;
        }
        let Some(cull_direction) = quad.cullface.as_deref().and_then(Direction::from_name) else {
            return false;
        };
        let (dx, dy, dz) = cull_direction.step();
        let neighbor_pos = BlockPos::new(block_pos.x + dx, block_pos.y + dy, block_pos.z + dz);
        let Some(neighbor_state) = grid.get_block_state_ref(&neighbor_pos) else {
            return false;
        };
        let key = (
            block_state as *const BlockState as usize,
            neighbor_state as *const BlockState as usize,
            cull_direction,
        );
        *cache.entry(key).or_insert_with(|| {
            !Self::should_render_face(block_state, neighbor_state, cull_direction)
        })
    }
    fn debug_disable_face_culling() -> bool {
        *DISABLE_FACE_CULLING.get_or_init(|| {
            std::env::var("ELTHARN_DISABLE_FACE_CULL")
                .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
                .unwrap_or(false)
        })
    }
    fn same_fluid(left: &BlockState, right: &BlockState) -> bool {
        let left = left.get_fluid_state();
        let right = right.get_fluid_state();
        !left.is_empty() && left.kind == right.kind
    }
    fn quad_should_use_translucent_layer(
        block_state: &BlockState,
        quad: &crate::render::baked_model::BakedQuad,
    ) -> bool {
        if !quad.layer.translucent() {
            return false;
        }
        Self::block_uses_translucent_render_layer(block_state.block.path.as_str())
    }
    pub(crate) fn block_uses_translucent_render_layer(path: &str) -> bool {
        matches!(
            path,
            "water"
                | "lava"
                | "ice"
                | "frosted_ice"
                | "slime_block"
                | "honey_block"
                | "glass"
                | "glass_pane"
                | "tinted_glass"
        ) || path.ends_with("_stained_glass")
            || path.ends_with("_stained_glass_pane")
    }
    fn should_render_face(
        state: &BlockState,
        neighbor_state: &BlockState,
        direction: Direction,
    ) -> bool {
        if matches!(
            neighbor_state.block.path.as_str(),
            "" | "air" | "cave_air" | "void_air"
        ) {
            return true;
        }
        if Self::state_skips_rendering(state, neighbor_state, direction) {
            return false;
        }
        let shape = state.get_occlusion_shape();
        let face = Self::face_rectangles(&shape, direction);
        if face.is_empty() {
            return true;
        }
        let neighbor_shape = neighbor_state.get_occlusion_shape();
        let neighbor_face = Self::face_rectangles(&neighbor_shape, direction.opposite());
        if neighbor_face.is_empty() {
            return true;
        }
        Self::rectangles_only_first_is_not_empty(&face, &neighbor_face)
    }
    fn state_skips_rendering(
        state: &BlockState,
        neighbor_state: &BlockState,
        direction: Direction,
    ) -> bool {
        if matches!(
            state.block.path.as_str(),
            "water" | "lava" | "flowing_water" | "flowing_lava"
        ) && Self::same_fluid(state, neighbor_state)
        {
            return true;
        }
        let name = state.block.path.as_str();
        let neighbor_name = neighbor_state.block.path.as_str();
        let same_block = state.block == neighbor_state.block;
        if same_block
            && (name.contains("glass")
                || name == "ice"
                || name == "frosted_ice"
                || name == "slime_block"
                || name == "honey_block"
                || name == "powder_snow")
        {
            return true;
        }
        if same_block && name == "mangrove_roots" {
            return matches!(direction, Direction::Up | Direction::Down);
        }
        let is_bars = |path: &str| path == "iron_bars" || path.ends_with("_bars");
        if (same_block || is_bars(name) && is_bars(neighbor_name))
            && neighbor_state
                .properties
                .contains_key(direction.opposite().name())
        {
            if matches!(direction, Direction::Up | Direction::Down) {
                return true;
            }
            return state
                .get_property(direction.name())
                .is_some_and(|value| value == "true")
                && neighbor_state
                    .get_property(direction.opposite().name())
                    .is_some_and(|value| value == "true");
        }
        false
    }
    fn baked_model_for_state<'a>(
        block_state: &BlockState,
        model_cache: &'a HashMap<String, BakedModel>,
    ) -> Option<&'a BakedModel> {
        let model_key = Self::block_state_model_key(block_state);
        model_cache
            .get(&model_key)
            .or_else(|| model_cache.get(&block_state.block.to_string()))
            .or_else(|| model_cache.get(&block_state.block.path))
    }
    pub(crate) fn occlusion_shape_covers_face(shape: &VoxelShape, direction: Direction) -> bool {
        Self::rectangles_cover_unit_face(&Self::face_rectangles(shape, direction))
    }
    fn occlusion_shape_is_full_block(shape: &VoxelShape) -> bool {
        Direction::VALUES
            .into_iter()
            .all(|direction| Self::occlusion_shape_covers_face(shape, direction))
    }
    fn face_rectangles(shape: &VoxelShape, direction: Direction) -> Vec<[f64; 4]> {
        const EPS: f64 = 1.0e-7;
        shape
            .to_aabbs()
            .iter()
            .filter_map(|bounds| {
                let touches_face = match direction {
                    Direction::Down => bounds.min_y <= EPS,
                    Direction::Up => bounds.max_y >= 1.0 - EPS,
                    Direction::North => bounds.min_z <= EPS,
                    Direction::South => bounds.max_z >= 1.0 - EPS,
                    Direction::West => bounds.min_x <= EPS,
                    Direction::East => bounds.max_x >= 1.0 - EPS,
                };
                if !touches_face {
                    return None;
                }
                let (u0, v0, u1, v1) = match direction {
                    Direction::Down | Direction::Up => {
                        (bounds.min_x, bounds.min_z, bounds.max_x, bounds.max_z)
                    }
                    Direction::North | Direction::South => {
                        (bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y)
                    }
                    Direction::West | Direction::East => {
                        (bounds.min_z, bounds.min_y, bounds.max_z, bounds.max_y)
                    }
                };
                let rectangle = [
                    u0.clamp(0.0, 1.0),
                    v0.clamp(0.0, 1.0),
                    u1.clamp(0.0, 1.0),
                    v1.clamp(0.0, 1.0),
                ];
                (rectangle[2] - rectangle[0] > EPS && rectangle[3] - rectangle[1] > EPS)
                    .then_some(rectangle)
            })
            .collect()
    }
    fn rectangles_cover_unit_face(rectangles: &[[f64; 4]]) -> bool {
        !Self::rectangles_only_first_is_not_empty(&[[0.0, 0.0, 1.0, 1.0]], rectangles)
    }
    fn rectangles_only_first_is_not_empty(first: &[[f64; 4]], second: &[[f64; 4]]) -> bool {
        if first.is_empty() {
            return false;
        }
        let mut us = vec![0.0, 1.0];
        let mut vs = vec![0.0, 1.0];
        for rectangle in first.iter().chain(second.iter()) {
            us.extend_from_slice(&[rectangle[0], rectangle[2]]);
            vs.extend_from_slice(&[rectangle[1], rectangle[3]]);
        }
        us.sort_by(f64::total_cmp);
        vs.sort_by(f64::total_cmp);
        us.dedup_by(|left, right| (*left - *right).abs() < 1.0e-7);
        vs.dedup_by(|left, right| (*left - *right).abs() < 1.0e-7);
        let contains = |rectangles: &[[f64; 4]], u: f64, v: f64| {
            rectangles.iter().any(|rectangle| {
                u > rectangle[0] && u < rectangle[2] && v > rectangle[1] && v < rectangle[3]
            })
        };
        us.windows(2).any(|u| {
            vs.windows(2).any(|v| {
                let midpoint_u = (u[0] + u[1]) * 0.5;
                let midpoint_v = (v[0] + v[1]) * 0.5;
                contains(first, midpoint_u, midpoint_v) && !contains(second, midpoint_u, midpoint_v)
            })
        })
    }
    fn block_state_from_chunk_snapshot(
        chunk: &Chunk,
        neighbor_chunks: &[&Chunk],
        pos: &BlockPos,
    ) -> Option<BlockState> {
        let cx = pos.x.div_euclid(16);
        let cz = pos.z.div_euclid(16);
        if cx == chunk.x && cz == chunk.z {
            return chunk.get_block_state(pos);
        }
        neighbor_chunks
            .iter()
            .find(|neighbor| neighbor.x == cx && neighbor.z == cz)
            .and_then(|neighbor| neighbor.get_block_state(pos))
    }
}
fn terrain_cardinal_normal_for_baked_quad(
    quad: &crate::render::baked_model::BakedQuad,
) -> [f32; 3] {
    if !quad.shade {
        return [0.0, 1.0, 0.0];
    }
    match quad.direction {
        Direction::Down => [0.0, -1.0, 0.0],
        Direction::Up => [0.0, 1.0, 0.0],
        Direction::North => [0.0, 0.0, -1.0],
        Direction::South => [0.0, 0.0, 1.0],
        Direction::West => [-1.0, 0.0, 0.0],
        Direction::East => [1.0, 0.0, 0.0],
    }
}
fn is_java_26_2_leaves_block(block_state: &BlockState) -> bool {
    if block_state.block.namespace != "minecraft" {
        return false;
    }
    matches!(
        block_state.block.path.as_str(),
        "oak_leaves"
            | "spruce_leaves"
            | "birch_leaves"
            | "jungle_leaves"
            | "acacia_leaves"
            | "cherry_leaves"
            | "dark_oak_leaves"
            | "pale_oak_leaves"
            | "mangrove_leaves"
            | "azalea_leaves"
            | "flowering_azalea_leaves"
    )
}
pub(crate) trait BlockView {
    fn get_block_state(&self, pos: BlockPos) -> BlockState;
    fn get_block_state_ref(&self, pos: BlockPos) -> &BlockState;
    fn brightness(&self, sky: bool, pos: BlockPos) -> u8;
}
#[inline]
fn default_air_state() -> &'static BlockState {
    static AIR: OnceLock<BlockState> = OnceLock::new();
    AIR.get_or_init(|| {
        BlockState::new(std::sync::Arc::new(
            el_core::resource_location::ResourceLocation::new("minecraft", "air"),
        ))
    })
}
struct MovingBlockRenderView<'a> {
    level: &'a ClientLevel,
    block_pos: BlockPos,
    block_state: &'a BlockState,
    biome: &'a Biome,
}
impl<'a> MovingBlockRenderView<'a> {
    fn new(level: &'a ClientLevel, block_pos: BlockPos, block_state: &'a BlockState) -> Self {
        let biome = level.get_biome(block_pos);
        Self {
            level,
            block_pos,
            block_state,
            biome,
        }
    }
}
impl BlockView for MovingBlockRenderView<'_> {
    fn get_block_state(&self, pos: BlockPos) -> BlockState {
        if pos == self.block_pos {
            self.block_state.clone()
        } else {
            self.level.air.clone()
        }
    }
    #[inline(always)]
    fn get_block_state_ref(&self, pos: BlockPos) -> &BlockState {
        if pos == self.block_pos {
            self.block_state
        } else {
            &self.level.air
        }
    }
    fn brightness(&self, sky: bool, pos: BlockPos) -> u8 {
        self.level.brightness(sky, pos)
    }
}
impl BlockAndTintGetter for MovingBlockRenderView<'_> {
    fn colormaps(&self) -> &ClientBlockColors {
        &self.level.block_colors
    }
    fn get_biome(&self, _pos: BlockPos) -> &Biome {
        self.biome
    }
    fn blend_radius(&self) -> i32 {
        0
    }
}
struct LevelBlockView<'a> {
    chunk: &'a Chunk,
    level: Option<&'a ClientLevel>,
}
impl<'a> BlockView for LevelBlockView<'a> {
    fn get_block_state(&self, pos: BlockPos) -> BlockState {
        if let Some(level) = self.level {
            level.get_block_state(pos)
        } else {
            self.chunk
                .get_block_state(&pos)
                .unwrap_or_else(|| default_air_state().clone())
        }
    }
    #[inline(always)]
    fn get_block_state_ref(&self, pos: BlockPos) -> &BlockState {
        if let Some(level) = self.level {
            level.get_block_state_ref(pos).unwrap_or(&level.air)
        } else {
            self.chunk
                .sections
                .get(((pos.y >> 4) - self.chunk.min_y_section) as usize)
                .map(|section| section.get_block_state_ref(pos.x, pos.y, pos.z))
                .unwrap_or_else(|| default_air_state())
        }
    }
    fn brightness(&self, sky: bool, pos: BlockPos) -> u8 {
        self.level
            .map(|level| level.brightness(sky, pos))
            .unwrap_or_else(|| self.chunk.brightness(sky, &pos))
    }
}
#[derive(Clone, Copy)]
pub struct NeighborGrid<'a> {
    center_x: i32,
    center_z: i32,
    grid: [Option<&'a Chunk>; 9],
}
impl<'a> NeighborGrid<'a> {
    #[inline(always)]
    pub fn new(center: &'a Chunk, neighbors: &'a [&'a Chunk]) -> Self {
        let mut grid = [None; 9];
        grid[1 + 1 * 3] = Some(center);
        for &neighbor in neighbors {
            let dx = neighbor.x - center.x;
            let dz = neighbor.z - center.z;
            if (dx as u32).wrapping_add(1) <= 2 && (dz as u32).wrapping_add(1) <= 2 {
                let idx = (dx + 1) as usize + (dz + 1) as usize * 3;
                grid[idx] = Some(neighbor);
            }
        }
        Self {
            center_x: center.x,
            center_z: center.z,
            grid,
        }
    }
    #[inline(always)]
    pub fn get_chunk(&self, cx: i32, cz: i32) -> Option<&'a Chunk> {
        let dx = cx - self.center_x;
        let dz = cz - self.center_z;
        if (dx as u32).wrapping_add(1) <= 2 && (dz as u32).wrapping_add(1) <= 2 {
            let idx = (dx + 1) as usize + (dz + 1) as usize * 3;
            self.grid[idx]
        } else {
            None
        }
    }
    #[inline(always)]
    pub fn get_block_state(&self, pos: &BlockPos) -> Option<BlockState> {
        let cx = pos.x >> 4;
        let cz = pos.z >> 4;
        self.get_chunk(cx, cz).and_then(|c| c.get_block_state(pos))
    }
    #[inline(always)]
    pub fn get_block_state_ref(&self, pos: &BlockPos) -> Option<&'a BlockState> {
        let cx = pos.x >> 4;
        let cz = pos.z >> 4;
        self.get_chunk(cx, cz).and_then(|chunk| {
            let section = (pos.y >> 4) - chunk.min_y_section;
            chunk
                .sections
                .get(section as usize)
                .map(|section| section.get_block_state_ref(pos.x, pos.y, pos.z))
        })
    }
    #[inline(always)]
    pub fn brightness(&self, sky: bool, pos: BlockPos) -> u8 {
        let cx = pos.x >> 4;
        let cz = pos.z >> 4;
        self.get_chunk(cx, cz)
            .map(|c| c.brightness(sky, &pos))
            .unwrap_or(if sky { 15 } else { 0 })
    }
}
struct NeighborSnapshotBlockView<'a> {
    grid: NeighborGrid<'a>,
}
impl<'a> BlockView for NeighborSnapshotBlockView<'a> {
    #[inline(always)]
    fn get_block_state(&self, pos: BlockPos) -> BlockState {
        self.grid
            .get_block_state(&pos)
            .unwrap_or_else(|| default_air_state().clone())
    }
    #[inline(always)]
    fn get_block_state_ref(&self, pos: BlockPos) -> &BlockState {
        self.grid
            .get_block_state_ref(&pos)
            .unwrap_or_else(|| default_air_state())
    }
    #[inline(always)]
    fn brightness(&self, sky: bool, pos: BlockPos) -> u8 {
        self.grid.brightness(sky, pos)
    }
}
struct CachedSectionBlockView<'a, V> {
    inner: &'a V,
    brightness_cache: &'a RefCell<SectionBrightnessCache>,
}
impl<V: BlockView> BlockView for CachedSectionBlockView<'_, V> {
    #[inline(always)]
    fn get_block_state(&self, pos: BlockPos) -> BlockState {
        self.inner.get_block_state(pos)
    }
    #[inline(always)]
    fn get_block_state_ref(&self, pos: BlockPos) -> &BlockState {
        self.inner.get_block_state_ref(pos)
    }
    #[inline(always)]
    fn brightness(&self, sky: bool, pos: BlockPos) -> u8 {
        let sample = self
            .brightness_cache
            .borrow_mut()
            .get_or_insert_with(pos, || {
                [
                    self.inner.brightness(false, pos),
                    self.inner.brightness(true, pos),
                ]
            });
        sample[usize::from(sky)]
    }
}
struct SnapshotTintView<'a> {
    grid: NeighborGrid<'a>,
    block_colors: &'a ClientBlockColors,
    biome_definitions: &'a HashMap<String, crate::render::block_colors::Biome>,
    biome_blend_radius: i32,
    biome_zoom_seed: i64,
}
impl BlockAndTintGetter for SnapshotTintView<'_> {
    fn colormaps(&self) -> &ClientBlockColors {
        self.block_colors
    }
    fn get_biome(&self, pos: BlockPos) -> &crate::render::block_colors::Biome {
        let (quart_x, quart_y, quart_z) =
            crate::render::block_colors::java_biome_manager_quart(self.biome_zoom_seed, pos);
        let selected_pos = BlockPos::new(quart_x << 2, quart_y << 2, quart_z << 2);
        let cx = selected_pos.x >> 4;
        let cz = selected_pos.z >> 4;
        let chunk = self.grid.get_chunk(cx, cz);
        if let Some(biome) = chunk
            .and_then(|chunk| chunk.get_biome(&selected_pos))
            .and_then(|id| self.biome_definitions.get(&id.to_string()))
        {
            return biome;
        }
        static WARNED: std::sync::Once = std::sync::Once::new();
        if let Some(biome) = self.biome_definitions.get("minecraft:plains") {
            return biome;
        }
        WARNED.call_once(|| {
            log::warn!(
                "Biome snapshot has no minecraft:plains entry; tinting with another loaded biome"
            );
        });
        self.biome_definitions
            .values()
            .next()
            .unwrap_or_else(|| neutral_biome())
    }
    fn blend_radius(&self) -> i32 {
        self.biome_blend_radius.clamp(0, 7)
    }
    fn biome_zoom_seed(&self) -> i64 {
        self.biome_zoom_seed
    }
}
fn neutral_biome() -> &'static crate::render::block_colors::Biome {
    static NEUTRAL_BIOME: std::sync::OnceLock<crate::render::block_colors::Biome> =
        std::sync::OnceLock::new();
    NEUTRAL_BIOME.get_or_init(crate::render::block_colors::Biome::plains)
}
struct QuadShape {
    face_cubic: bool,
    face_partial: bool,
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
}
fn prepare_quad_shape(
    state: &BlockState,
    quad: &crate::render::baked_model::BakedQuad,
) -> QuadShape {
    let full_collision = ChunkMesher::occlusion_shape_is_full_block(&state.get_collision_shape());
    let mut min = glam::Vec3::splat(f32::INFINITY);
    let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
    for vertex in &quad.vertex_positions {
        min = min.min(*vertex);
        max = max.max(*vertex);
    }
    let (min_x, min_y, min_z) = (min.x, min.y, min.z);
    let (max_x, max_y, max_z) = (max.x, max.y, max.z);
    let face_cubic = match quad.direction {
        Direction::Down => min_y == max_y && (min_y < 0.0001 || full_collision),
        Direction::Up => min_y == max_y && (max_y > 0.9999 || full_collision),
        Direction::North => min_z == max_z && (min_z < 0.0001 || full_collision),
        Direction::South => min_z == max_z && (max_z > 0.9999 || full_collision),
        Direction::West => min_x == max_x && (min_x < 0.0001 || full_collision),
        Direction::East => max_x == min_x && (max_x > 0.9999 || full_collision),
    };
    let face_partial = match quad.direction {
        Direction::Down | Direction::Up => {
            min_x >= 0.0001 || min_z >= 0.0001 || max_x <= 0.9999 || max_z <= 0.9999
        }
        Direction::North | Direction::South => {
            min_x >= 0.0001 || min_y >= 0.0001 || max_x <= 0.9999 || max_y <= 0.9999
        }
        Direction::West | Direction::East => {
            min_y >= 0.0001 || min_z >= 0.0001 || max_y <= 0.9999 || max_z <= 0.9999
        }
    };
    QuadShape {
        face_cubic,
        face_partial,
        min_x,
        min_y,
        min_z,
        max_x,
        max_y,
        max_z,
    }
}
fn quad_vertex_lighting(
    level: &dyn BlockView,
    pos: BlockPos,
    state: &BlockState,
    quad: &crate::render::baked_model::BakedQuad,
    ambient_occlusion_enabled: bool,
) -> ([f32; 4], [[f32; 2]; 4]) {
    let QuadShape {
        face_cubic,
        face_partial,
        min_x,
        min_y,
        min_z,
        max_x,
        max_y,
        max_z,
    } = prepare_quad_shape(state, quad);
    let mut ao = [1.0; 4];
    let mut light = [[0.0; 2]; 4];
    if !should_use_ambient_occlusion(ambient_occlusion_enabled, state, quad) {
        let light_pos = if face_cubic {
            offset_pos(pos, quad.direction)
        } else {
            pos
        };
        light.fill(packed_light_to_uv(raw_light(level, state, light_pos)));
        apply_material_light_emission(&mut light, quad.light_emission);
        return (ao, light);
    }
    let info = ao_adjacency(quad.direction);
    let base = if face_cubic {
        offset_pos(pos, quad.direction)
    } else {
        pos
    };
    let next = offset_pos(pos, quad.direction);
    let next_state = level.get_block_state_ref(next);
    let center_light_pos = if face_cubic || !is_solid_render(&next_state) {
        next
    } else {
        pos
    };
    let center_light_state = if face_cubic || !is_solid_render(&next_state) {
        &next_state
    } else {
        state
    };
    let center_light = raw_light(level, center_light_state, center_light_pos);
    let shade_center =
        shade_brightness(level.get_block_state_ref(if face_cubic { base } else { pos }));
    let mut side_positions = [pos; 4];
    let mut side_shades = [1.0_f32; 4];
    let mut side_lights = [[0_u8; 2]; 4];
    let mut translucent = [false; 4];
    for i in 0..4 {
        let side_pos = offset_pos(base, info.corners[i]);
        let state = level.get_block_state_ref(side_pos);
        let forward = level.get_block_state_ref(offset_pos(side_pos, quad.direction));
        side_positions[i] = side_pos;
        side_shades[i] = shade_brightness(state);
        side_lights[i] = raw_light(level, state, side_pos);
        translucent[i] = !is_ao_occluder(forward);
    }
    let mut corner_shades = [1.0_f32; 4];
    let mut corner_lights = [[0_u8; 2]; 4];
    let corner_pairs = [(0_usize, 2_usize), (0, 3), (1, 2), (1, 3)];
    for (index, (a, b)) in corner_pairs.into_iter().enumerate() {
        if !translucent[a] && !translucent[b] {
            corner_shades[index] = side_shades[0];
            corner_lights[index] = side_lights[0];
        } else {
            let diagonal = offset_pos(side_positions[a], info.corners[b]);
            let diagonal_state = level.get_block_state_ref(diagonal);
            corner_shades[index] = shade_brightness(diagonal_state);
            corner_lights[index] = raw_light(level, diagonal_state, diagonal);
        }
    }
    let shade_levels = [
        (side_shades[3] + side_shades[0] + corner_shades[1] + shade_center) * 0.25,
        (side_shades[2] + side_shades[0] + corner_shades[0] + shade_center) * 0.25,
        (side_shades[2] + side_shades[1] + corner_shades[2] + shade_center) * 0.25,
        (side_shades[3] + side_shades[1] + corner_shades[3] + shade_center) * 0.25,
    ];
    let light_samples = [
        smooth_light(
            side_lights[3],
            side_lights[0],
            corner_lights[1],
            center_light,
        ),
        smooth_light(
            side_lights[2],
            side_lights[0],
            corner_lights[0],
            center_light,
        ),
        smooth_light(
            side_lights[2],
            side_lights[1],
            corner_lights[2],
            center_light,
        ),
        smooth_light(
            side_lights[3],
            side_lights[1],
            corner_lights[3],
            center_light,
        ),
    ];
    let remap = ambient_vertex_remap(quad.direction);
    if face_partial && info.do_non_cubic_weight {
        let shape = [
            min_x,
            max_x,
            min_y,
            max_y,
            min_z,
            max_z,
            1.0 - min_x,
            1.0 - max_x,
            1.0 - min_y,
            1.0 - max_y,
            1.0 - min_z,
            1.0 - max_z,
        ];
        for vertex in 0..4 {
            let weights = info.weights[vertex];
            let pair_weights = [
                shape[weights[0] as usize] * shape[weights[1] as usize],
                shape[weights[2] as usize] * shape[weights[3] as usize],
                shape[weights[4] as usize] * shape[weights[5] as usize],
                shape[weights[6] as usize] * shape[weights[7] as usize],
            ];
            let shade = if pair_weights.iter().sum::<f32>() <= f32::EPSILON {
                1.0
            } else {
                (shade_levels[0] * pair_weights[0]
                    + shade_levels[1] * pair_weights[1]
                    + shade_levels[2] * pair_weights[2]
                    + shade_levels[3] * pair_weights[3])
                    .clamp(0.0, 1.0)
            };
            ao[remap[vertex]] = shade;
            let blended = smooth_weighted_light(
                light_samples[0],
                light_samples[1],
                light_samples[2],
                light_samples[3],
                pair_weights,
            );
            light[remap[vertex]] = blended;
        }
    } else {
        for (canonical, actual) in remap.into_iter().enumerate() {
            ao[actual] = shade_levels[canonical].clamp(0.0, 1.0);
            light[actual] = light_samples[canonical];
        }
    }
    apply_material_light_emission(&mut light, quad.light_emission);
    (ao, light)
}
fn should_use_ambient_occlusion(
    ambient_occlusion_enabled: bool,
    state: &BlockState,
    quad: &crate::render::baked_model::BakedQuad,
) -> bool {
    ambient_occlusion_enabled && quad.ambient_occlusion && state_light_emission(state) == 0
}
fn state_light_emission(state: &BlockState) -> u8 {
    java_26_2_vanilla_block_lighting_properties(state)
        .map(|properties| properties.light_emission)
        .unwrap_or(0)
}
fn raw_light(level: &dyn BlockView, state: &BlockState, pos: BlockPos) -> [u8; 2] {
    if java_26_2_vanilla_block_lighting_properties(state)
        .is_some_and(|properties| properties.emissive_rendering)
    {
        return [240, 240];
    }
    [
        level
            .brightness(false, pos)
            .max(state_light_emission(state))
            .saturating_mul(16),
        level.brightness(true, pos).saturating_mul(16),
    ]
}
fn packed_light_to_uv(light: [u8; 2]) -> [f32; 2] {
    [f32::from(light[0]) / 256.0, f32::from(light[1]) / 256.0]
}
fn apply_material_light_emission(light: &mut [[f32; 2]; 4], emission: u8) {
    if emission == 0 {
        return;
    }
    for coords in light {
        let block = (coords[0] * 16.0).floor().clamp(0.0, 15.0) as u8;
        let sky = (coords[1] * 16.0).floor().clamp(0.0, 15.0) as u8;
        *coords = [
            f32::from(block.max(emission)) / 16.0,
            f32::from(sky.max(emission)) / 16.0,
        ];
    }
}
fn smooth_light(
    mut first: [u8; 2],
    mut second: [u8; 2],
    mut third: [u8; 2],
    center: [u8; 2],
) -> [f32; 2] {
    if center[0] > 32 || center[1] > 32 {
        for neighbor in [&mut first, &mut second, &mut third] {
            if neighbor[0] == 0 && neighbor[1] == 0 {
                *neighbor = center;
            } else if neighbor[1] == 0 {
                neighbor[1] = center[1];
            }
        }
    }
    [
        ((u16::from(first[0]) + u16::from(second[0]) + u16::from(third[0]) + u16::from(center[0]))
            / 4) as f32
            / 256.0,
        ((u16::from(first[1]) + u16::from(second[1]) + u16::from(third[1]) + u16::from(center[1]))
            / 4) as f32
            / 256.0,
    ]
}
fn smooth_weighted_light(
    first: [f32; 2],
    second: [f32; 2],
    third: [f32; 2],
    fourth: [f32; 2],
    weights: [f32; 4],
) -> [f32; 2] {
    [
        (first[0] * 256.0 * weights[0]
            + second[0] * 256.0 * weights[1]
            + third[0] * 256.0 * weights[2]
            + fourth[0] * 256.0 * weights[3])
            .floor()
            / 256.0,
        (first[1] * 256.0 * weights[0]
            + second[1] * 256.0 * weights[1]
            + third[1] * 256.0 * weights[2]
            + fourth[1] * 256.0 * weights[3])
            .floor()
            / 256.0,
    ]
}
#[derive(Clone, Copy)]
struct AoAdjacency {
    corners: [Direction; 4],
    do_non_cubic_weight: bool,
    weights: [[u8; 8]; 4],
}
fn ao_adjacency(direction: Direction) -> AoAdjacency {
    use Direction::{Down, East, North, South, Up, West};
    match direction {
        Down => AoAdjacency {
            corners: [West, East, North, South],
            do_non_cubic_weight: true,
            weights: [
                [6, 5, 6, 11, 0, 11, 0, 5],
                [6, 4, 6, 10, 0, 10, 0, 4],
                [7, 4, 7, 10, 1, 10, 1, 4],
                [7, 5, 7, 11, 1, 11, 1, 5],
            ],
        },
        Up => AoAdjacency {
            corners: [East, West, North, South],
            do_non_cubic_weight: true,
            weights: [
                [1, 5, 1, 11, 7, 11, 7, 5],
                [1, 4, 1, 10, 7, 10, 7, 4],
                [0, 4, 0, 10, 6, 10, 6, 4],
                [0, 5, 0, 11, 6, 11, 6, 5],
            ],
        },
        North => AoAdjacency {
            corners: [Up, Down, East, West],
            do_non_cubic_weight: true,
            weights: [
                [3, 6, 3, 0, 9, 0, 9, 6],
                [3, 7, 3, 1, 9, 1, 9, 7],
                [2, 7, 2, 1, 8, 1, 8, 7],
                [2, 6, 2, 0, 8, 0, 8, 6],
            ],
        },
        South => AoAdjacency {
            corners: [West, East, Down, Up],
            do_non_cubic_weight: true,
            weights: [
                [3, 6, 9, 6, 9, 0, 3, 0],
                [2, 6, 8, 6, 8, 0, 2, 0],
                [2, 7, 8, 7, 8, 1, 2, 1],
                [3, 7, 9, 7, 9, 1, 3, 1],
            ],
        },
        West => AoAdjacency {
            corners: [Up, Down, North, South],
            do_non_cubic_weight: true,
            weights: [
                [3, 5, 3, 11, 9, 11, 9, 5],
                [3, 4, 3, 10, 9, 10, 9, 4],
                [2, 4, 2, 10, 8, 10, 8, 4],
                [2, 5, 2, 11, 8, 11, 8, 5],
            ],
        },
        East => AoAdjacency {
            corners: [Down, Up, North, South],
            do_non_cubic_weight: true,
            weights: [
                [8, 5, 8, 11, 2, 11, 2, 5],
                [8, 4, 8, 10, 2, 10, 2, 4],
                [9, 4, 9, 10, 3, 10, 3, 4],
                [9, 5, 9, 11, 3, 11, 3, 5],
            ],
        },
    }
}
fn ambient_vertex_remap(direction: Direction) -> [usize; 4] {
    match direction {
        Direction::Down => [0, 1, 2, 3],
        Direction::Up => [2, 3, 0, 1],
        Direction::North => [3, 0, 1, 2],
        Direction::South => [0, 1, 2, 3],
        Direction::West => [3, 0, 1, 2],
        Direction::East => [1, 2, 3, 0],
    }
}
fn is_ao_occluder(state: &BlockState) -> bool {
    if let Some(properties) = java_26_2_vanilla_block_lighting_properties(state) {
        return properties.view_blocking && properties.light_dampening != 0;
    }
    let name = state.block.path.as_str();
    if name.contains("glass")
        || name.ends_with("_leaves")
        || matches!(
            name,
            "mangrove_roots"
                | "moving_piston"
                | "test_instance_block"
                | "copper_grate"
                | "exposed_copper_grate"
                | "weathered_copper_grate"
                | "oxidized_copper_grate"
                | "waxed_copper_grate"
                | "waxed_exposed_copper_grate"
                | "waxed_weathered_copper_grate"
                | "waxed_oxidized_copper_grate"
                | "trial_spawner"
                | "vault"
        )
    {
        return false;
    }
    if matches!(name, "farmland" | "dirt_path" | "mud" | "soul_sand") {
        return true;
    }
    if name == "snow" {
        return state
            .get_property("layers")
            .and_then(|layers| layers.parse::<i32>().ok())
            .is_some_and(|layers| layers >= 8);
    }
    let collision = state.get_collision_shape();
    !collision.is_empty() && ChunkMesher::occlusion_shape_is_full_block(&collision)
}
fn is_solid_render(state: &BlockState) -> bool {
    if let Some(properties) = java_26_2_vanilla_block_lighting_properties(state) {
        return properties.solid_render;
    }
    ChunkMesher::occlusion_shape_is_full_block(&state.get_occlusion_shape())
}
fn offset_pos(pos: BlockPos, direction: Direction) -> BlockPos {
    let (x, y, z) = direction.step();
    BlockPos::new(pos.x + x, pos.y + y, pos.z + z)
}
fn shade_brightness(state: &BlockState) -> f32 {
    if let Some(properties) = java_26_2_vanilla_block_lighting_properties(state) {
        return properties.shade_brightness;
    }
    if matches!(
        state.block.path.as_str(),
        "glass" | "tinted_glass" | "barrier" | "light" | "structure_void"
    ) || state.block.path.ends_with("_stained_glass")
    {
        1.0
    } else if matches!(state.block.path.as_str(), "mud" | "soul_sand") {
        0.2
    } else if state.block.path == "snow" {
        if state
            .get_property("layers")
            .is_some_and(|layers| layers == "8")
        {
            0.2
        } else {
            1.0
        }
    } else if ChunkMesher::occlusion_shape_is_full_block(&state.get_collision_shape()) {
        0.2
    } else {
        1.0
    }
}
fn fluid_matches_path(fluid: el_core::fluid_state::FluidState, fluid_path: &str) -> bool {
    match fluid_path {
        "water" => fluid.is_water(),
        "lava" => fluid.is_lava(),
        _ => false,
    }
}
fn fluid_light(level: &dyn BlockView, pos: BlockPos) -> [f32; 2] {
    let above = offset_pos(pos, Direction::Up);
    let here = raw_light(level, &level.get_block_state(pos), pos);
    let up = raw_light(level, &level.get_block_state(above), above);
    packed_light_to_uv([here[0].max(up[0]), here[1].max(up[1])])
}
fn fluid_state_is_solid(state: &BlockState) -> bool {
    if let Some(properties) = java_26_2_vanilla_block_fluid_support_properties(state) {
        return properties.solid;
    }
    !state.get_collision_shape().is_empty()
}
fn fluid_state_blocks_motion(state: &BlockState) -> bool {
    if let Some(properties) = java_26_2_vanilla_block_fluid_support_properties(state) {
        return properties.blocks_motion;
    }
    !state.get_collision_shape().is_empty()
}
fn is_fluid_falling(state: &BlockState) -> bool {
    state.get_fluid_state().falling
}
fn get_own_height(state: &BlockState, fluid_path: &str) -> f32 {
    let fluid = state.get_fluid_state();
    if fluid_matches_path(fluid, fluid_path) {
        fluid.own_height()
    } else {
        0.0
    }
}
fn get_height(level: &dyn BlockView, fluid_path: &str, pos: BlockPos, state: &BlockState) -> f32 {
    let fluid = state.get_fluid_state();
    if fluid_matches_path(fluid, fluid_path) {
        let above_state = level.get_block_state(BlockPos::new(pos.x, pos.y + 1, pos.z));
        if fluid_matches_path(above_state.get_fluid_state(), fluid_path) {
            1.0
        } else {
            fluid.own_height()
        }
    } else if fluid_state_is_solid(state) {
        -1.0
    } else {
        0.0
    }
}
fn calculate_average_height(
    level: &dyn BlockView,
    fluid_path: &str,
    height_self: f32,
    height1: f32,
    height2: f32,
    corner_pos: BlockPos,
) -> f32 {
    if height1 >= 1.0 || height2 >= 1.0 {
        1.0
    } else {
        let mut weighted_height = [0.0, 0.0];
        if height1 > 0.0 || height2 > 0.0 {
            let state = level.get_block_state(corner_pos);
            let height_corner = get_height(level, fluid_path, corner_pos, &state);
            if height_corner >= 1.0 {
                return 1.0;
            }
            add_weighted_height(&mut weighted_height, height_corner);
        }
        add_weighted_height(&mut weighted_height, height_self);
        add_weighted_height(&mut weighted_height, height1);
        add_weighted_height(&mut weighted_height, height2);
        weighted_height[0] / weighted_height[1]
    }
}
fn add_weighted_height(weighted_height: &mut [f32; 2], height: f32) {
    if height >= 0.8 {
        weighted_height[0] += height * 10.0;
        weighted_height[1] += 10.0;
    } else if height >= 0.0 {
        weighted_height[0] += height;
        weighted_height[1] += 1.0;
    }
}
fn is_solid_face(
    level: &dyn BlockView,
    pos: BlockPos,
    direction: el_math::Direction,
    fluid_path: &str,
) -> bool {
    let state = level.get_block_state(pos);
    if fluid_matches_path(state.get_fluid_state(), fluid_path) {
        return false;
    }
    if direction == el_math::Direction::Up {
        return true;
    }
    if let Some(properties) = java_26_2_vanilla_block_fluid_support_properties(&state) {
        if properties.ice_block {
            return false;
        }
        return properties.face_sturdy[java_direction_ordinal(direction)];
    }
    ChunkMesher::occlusion_shape_covers_face(&state.get_occlusion_shape(), direction)
}
fn java_direction_ordinal(direction: el_math::Direction) -> usize {
    match direction {
        el_math::Direction::Down => 0,
        el_math::Direction::Up => 1,
        el_math::Direction::North => 2,
        el_math::Direction::South => 3,
        el_math::Direction::West => 4,
        el_math::Direction::East => 5,
    }
}
fn is_face_occluded_by_state(
    direction: el_math::Direction,
    height: f32,
    state: &BlockState,
) -> bool {
    let occlusion_shape = state.get_occlusion_shape();
    if occlusion_shape.is_empty() {
        return false;
    }
    let occluder = ChunkMesher::face_rectangles(&occlusion_shape, direction.opposite());
    if occluder.is_empty() {
        return false;
    }
    if ChunkMesher::rectangles_cover_unit_face(&occluder) {
        return direction != el_math::Direction::Up || height == 1.0;
    }
    let fluid_shape =
        VoxelShape::from_aabb(el_math::AABB::new(0.0, 0.0, 0.0, 1.0, height as f64, 1.0));
    let fluid_face = ChunkMesher::face_rectangles(&fluid_shape, direction);
    !fluid_face.is_empty()
        && !ChunkMesher::rectangles_only_first_is_not_empty(&fluid_face, &occluder)
}
fn is_face_occluded_by_neighbor(
    direction: el_math::Direction,
    height: f32,
    neighbor_state: &BlockState,
) -> bool {
    is_face_occluded_by_state(direction, height, neighbor_state)
}
fn is_face_occluded_by_self(state: &BlockState, direction: el_math::Direction) -> bool {
    is_face_occluded_by_state(direction.opposite(), 1.0, state)
}
fn should_render_fluid_face(
    fluid_path: &str,
    block_state: &BlockState,
    direction: Direction,
    neighbor_state: &BlockState,
) -> bool {
    !fluid_matches_path(neighbor_state.get_fluid_state(), fluid_path)
        && !is_face_occluded_by_self(block_state, direction)
}
fn normalize_flow(flow: glam::Vec3) -> glam::Vec3 {
    let len = flow.length();
    if len < 1e-4 {
        glam::Vec3::ZERO
    } else {
        flow / len
    }
}
fn get_flow(
    level: &dyn BlockView,
    pos: BlockPos,
    state: &BlockState,
    fluid_path: &str,
) -> glam::Vec3 {
    let mut flow_x = 0.0;
    let mut flow_z = 0.0;
    let own_height = get_own_height(state, fluid_path);
    for direction in [
        el_math::Direction::North,
        el_math::Direction::South,
        el_math::Direction::West,
        el_math::Direction::East,
    ] {
        let (dx, _, dz) = direction.step();
        let neighbor_pos = BlockPos::new(pos.x + dx, pos.y, pos.z + dz);
        let neighbor_state = level.get_block_state(neighbor_pos);
        let neighbor_fluid = neighbor_state.get_fluid_state();
        let affects_flow =
            neighbor_fluid.is_empty() || fluid_matches_path(neighbor_fluid, fluid_path);
        if affects_flow {
            let neighbor_height = get_own_height(&neighbor_state, fluid_path);
            let mut distance = 0.0;
            if neighbor_height == 0.0 {
                if !fluid_state_blocks_motion(&neighbor_state) {
                    let below_neighbor_pos =
                        BlockPos::new(neighbor_pos.x, neighbor_pos.y - 1, neighbor_pos.z);
                    let below_neighbor_state = level.get_block_state(below_neighbor_pos);
                    let below_fluid = below_neighbor_state.get_fluid_state();
                    let below_affects =
                        below_fluid.is_empty() || fluid_matches_path(below_fluid, fluid_path);
                    if below_affects {
                        let below_height = get_own_height(&below_neighbor_state, fluid_path);
                        if below_height > 0.0 {
                            distance = own_height - (below_height - 0.8888889);
                        }
                    }
                }
            } else if neighbor_height > 0.0 {
                distance = own_height - neighbor_height;
            }
            if distance != 0.0 {
                flow_x += dx as f32 * distance;
                flow_z += dz as f32 * distance;
            }
        }
    }
    let mut flow = glam::Vec3::new(flow_x, 0.0, flow_z);
    if is_fluid_falling(state) {
        let mut adj_solid = false;
        for direction in [
            el_math::Direction::North,
            el_math::Direction::South,
            el_math::Direction::West,
            el_math::Direction::East,
        ] {
            let (dx, _, dz) = direction.step();
            let neighbor_pos = BlockPos::new(pos.x + dx, pos.y, pos.z + dz);
            let above_neighbor_pos =
                BlockPos::new(neighbor_pos.x, neighbor_pos.y + 1, neighbor_pos.z);
            if is_solid_face(level, neighbor_pos, direction, fluid_path)
                || is_solid_face(level, above_neighbor_pos, direction, fluid_path)
            {
                adj_solid = true;
                break;
            }
        }
        if adj_solid {
            flow = normalize_flow(flow) + glam::Vec3::new(0.0, -6.0, 0.0);
        }
    }
    normalize_flow(flow)
}
fn should_render_backward_up_face(
    level: &dyn BlockView,
    fluid_path: &str,
    above: BlockPos,
) -> bool {
    for ox in -1..=1 {
        for oz in -1..=1 {
            let offset = BlockPos::new(above.x + ox, above.y, above.z + oz);
            let state = level.get_block_state(offset);
            if !fluid_matches_path(state.get_fluid_state(), fluid_path) && !is_solid_render(&state)
            {
                return true;
            }
        }
    }
    false
}
fn tint_from_argb(color: u32) -> [f32; 4] {
    [
        ((color >> 16) & 0xFF) as f32 / 255.0,
        ((color >> 8) & 0xFF) as f32 / 255.0,
        (color & 0xFF) as f32 / 255.0,
        ((color >> 24) & 0xFF) as f32 / 255.0,
    ]
}
pub(crate) fn block_model_seed(block_state: &BlockState, pos: BlockPos) -> i64 {
    el_core::block_behaviour::java_26_2_vanilla_block_model_seed(block_state, pos)
        .unwrap_or_else(|| generic_java_block_model_seed(pos))
}
fn generic_java_block_model_seed(pos: BlockPos) -> i64 {
    let mut seed = i64::from(pos.x.wrapping_mul(3_129_871))
        ^ (pos.z as i64).wrapping_mul(116_129_781)
        ^ pos.y as i64;
    seed = seed
        .wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11));
    seed >> 16
}
fn add_face(
    vertices: &mut Vec<Vertex>,
    indices: &mut Vec<u32>,
    index_offset: &mut u32,
    p0: [f32; 3],
    uv0: [f32; 2],
    p1: [f32; 3],
    uv1: [f32; 2],
    p2: [f32; 3],
    uv2: [f32; 2],
    p3: [f32; 3],
    uv3: [f32; 2],
    color: [f32; 4],
    normal: [f32; 3],
    light: [f32; 2],
    add_back_face: bool,
) {
    vertices.push(Vertex {
        position: p0,
        tex_coords: uv0,
        normal,
        color,
        light,
    });
    vertices.push(Vertex {
        position: p1,
        tex_coords: uv1,
        normal,
        color,
        light,
    });
    vertices.push(Vertex {
        position: p2,
        tex_coords: uv2,
        normal,
        color,
        light,
    });
    vertices.push(Vertex {
        position: p3,
        tex_coords: uv3,
        normal,
        color,
        light,
    });
    indices.extend_from_slice(&[
        *index_offset,
        *index_offset + 1,
        *index_offset + 2,
        *index_offset + 2,
        *index_offset + 3,
        *index_offset,
    ]);
    *index_offset += 4;
    if add_back_face {
        vertices.push(Vertex {
            position: p0,
            tex_coords: uv0,
            normal,
            color,
            light,
        });
        vertices.push(Vertex {
            position: p3,
            tex_coords: uv3,
            normal,
            color,
            light,
        });
        vertices.push(Vertex {
            position: p2,
            tex_coords: uv2,
            normal,
            color,
            light,
        });
        vertices.push(Vertex {
            position: p1,
            tex_coords: uv1,
            normal,
            color,
            light,
        });
        indices.extend_from_slice(&[
            *index_offset,
            *index_offset + 1,
            *index_offset + 2,
            *index_offset + 2,
            *index_offset + 3,
            *index_offset,
        ]);
        *index_offset += 4;
    }
}
pub struct FluidRenderer;
impl FluidRenderer {
    pub fn tesselate(
        level: &dyn BlockView,
        pos: BlockPos,
        block_state: &BlockState,
        block_colors: &ClientBlockColors,
        tint_view: Option<&dyn BlockAndTintGetter>,
        vertices: &mut Vec<Vertex>,
        opaque_indices: &mut Vec<u32>,
        translucent_indices: &mut Vec<u32>,
        index_offset: &mut u32,
    ) {
        let fluid = block_state.get_fluid_state();
        if fluid.is_empty() {
            return;
        }
        let fluid_path = if fluid.is_water() { "water" } else { "lava" };
        let is_water = fluid_path == "water";
        let layer = if is_water {
            block_colors
                .water_layer
                .get()
                .copied()
                .unwrap_or(crate::render::chunk::ChunkSectionLayer::Translucent)
        } else {
            block_colors
                .lava_layer
                .get()
                .copied()
                .unwrap_or(crate::render::chunk::ChunkSectionLayer::Solid)
        };
        let indices = match layer {
            crate::render::chunk::ChunkSectionLayer::Translucent => translucent_indices,
            crate::render::chunk::ChunkSectionLayer::Solid
            | crate::render::chunk::ChunkSectionLayer::Cutout => opaque_indices,
        };
        let light = fluid_light(level, pos);
        let pos_down = BlockPos::new(pos.x, pos.y - 1, pos.z);
        let block_state_down = level.get_block_state(pos_down);
        let below_light = fluid_light(level, pos_down);
        let pos_up = BlockPos::new(pos.x, pos.y + 1, pos.z);
        let block_state_up = level.get_block_state(pos_up);
        let pos_north = BlockPos::new(pos.x, pos.y, pos.z - 1);
        let block_state_north = level.get_block_state(pos_north);
        let pos_south = BlockPos::new(pos.x, pos.y, pos.z + 1);
        let block_state_south = level.get_block_state(pos_south);
        let pos_west = BlockPos::new(pos.x - 1, pos.y, pos.z);
        let block_state_west = level.get_block_state(pos_west);
        let pos_east = BlockPos::new(pos.x + 1, pos.y, pos.z);
        let block_state_east = level.get_block_state(pos_east);
        let render_up = !fluid_matches_path(block_state_up.get_fluid_state(), fluid_path);
        let render_down = should_render_fluid_face(
            fluid_path,
            block_state,
            el_math::Direction::Down,
            &block_state_down,
        ) && !is_face_occluded_by_neighbor(
            el_math::Direction::Down,
            0.8888889,
            &block_state_down,
        );
        let render_north = should_render_fluid_face(
            fluid_path,
            block_state,
            el_math::Direction::North,
            &block_state_north,
        );
        let render_south = should_render_fluid_face(
            fluid_path,
            block_state,
            el_math::Direction::South,
            &block_state_south,
        );
        let render_west = should_render_fluid_face(
            fluid_path,
            block_state,
            el_math::Direction::West,
            &block_state_west,
        );
        let render_east = should_render_fluid_face(
            fluid_path,
            block_state,
            el_math::Direction::East,
            &block_state_east,
        );
        if !(render_up || render_down || render_east || render_west || render_north || render_south)
        {
            return;
        }
        let still_sprite = if is_water {
            block_colors.water_still_sprite.get().copied()
        } else {
            block_colors.lava_still_sprite.get().copied()
        };
        let flow_sprite = if is_water {
            block_colors.water_flow_sprite.get().copied()
        } else {
            block_colors.lava_flow_sprite.get().copied()
        };
        let overlay_sprite = if is_water {
            block_colors.water_overlay_sprite.get().copied()
        } else {
            None
        };
        let (Some(still_sprite), Some(flow_sprite)) = (still_sprite, flow_sprite) else {
            return;
        };
        let mut tint_color = if is_water {
            if let Some(view) = tint_view {
                tint_from_argb(view.get_average_water_color(pos))
            } else {
                tint_from_argb(0xFF3F76E4)
            }
        } else {
            [1.0, 1.0, 1.0, 1.0]
        };
        if is_water {
            tint_color[3] = 1.0;
        }
        let height_self = get_height(level, fluid_path, pos, block_state);
        let (h_ne, h_nw, h_se, h_sw) = if height_self >= 1.0 {
            (1.0, 1.0, 1.0, 1.0)
        } else {
            let h_north = get_height(level, fluid_path, pos_north, &block_state_north);
            let h_south = get_height(level, fluid_path, pos_south, &block_state_south);
            let h_east = get_height(level, fluid_path, pos_east, &block_state_east);
            let h_west = get_height(level, fluid_path, pos_west, &block_state_west);
            (
                calculate_average_height(
                    level,
                    fluid_path,
                    height_self,
                    h_north,
                    h_east,
                    BlockPos::new(pos.x + 1, pos.y, pos.z - 1),
                ),
                calculate_average_height(
                    level,
                    fluid_path,
                    height_self,
                    h_north,
                    h_west,
                    BlockPos::new(pos.x - 1, pos.y, pos.z - 1),
                ),
                calculate_average_height(
                    level,
                    fluid_path,
                    height_self,
                    h_south,
                    h_east,
                    BlockPos::new(pos.x + 1, pos.y, pos.z + 1),
                ),
                calculate_average_height(
                    level,
                    fluid_path,
                    height_self,
                    h_south,
                    h_west,
                    BlockPos::new(pos.x - 1, pos.y, pos.z + 1),
                ),
            )
        };
        let x = pos.x as f32;
        let y = pos.y as f32;
        let z = pos.z as f32;
        let bottom_offs = if render_down { 0.001 } else { 0.0 };
        if render_up {
            let min_h = h_nw.min(h_sw).min(h_se).min(h_ne);
            if !is_face_occluded_by_neighbor(el_math::Direction::Up, min_h, &block_state_up) {
                let h_nw_adj = h_nw - 0.001;
                let h_sw_adj = h_sw - 0.001;
                let h_se_adj = h_se - 0.001;
                let h_ne_adj = h_ne - 0.001;
                let flow = get_flow(level, pos, block_state, fluid_path);
                let (uv00, uv01, uv10, uv11) = if flow.x == 0.0 && flow.z == 0.0 {
                    (
                        still_sprite.map_unit_uv(0.0, 0.0),
                        still_sprite.map_unit_uv(0.0, 1.0),
                        still_sprite.map_unit_uv(1.0, 1.0),
                        still_sprite.map_unit_uv(1.0, 0.0),
                    )
                } else {
                    let angle = flow.z.atan2(flow.x) - (std::f32::consts::PI / 2.0);
                    let s = angle.sin() * 0.25;
                    let c = angle.cos() * 0.25;
                    (
                        flow_sprite.map_unit_uv(0.5 + (-c - s), 0.5 + (-c + s)),
                        flow_sprite.map_unit_uv(0.5 + (-c + s), 0.5 + (c + s)),
                        flow_sprite.map_unit_uv(0.5 + (c + s), 0.5 + (c - s)),
                        flow_sprite.map_unit_uv(0.5 + (c - s), 0.5 + (-c - s)),
                    )
                };
                add_face(
                    vertices,
                    indices,
                    index_offset,
                    [x + 0.0, y + h_nw_adj, z + 0.0],
                    uv00,
                    [x + 0.0, y + h_sw_adj, z + 1.0],
                    uv01,
                    [x + 1.0, y + h_se_adj, z + 1.0],
                    uv10,
                    [x + 1.0, y + h_ne_adj, z + 0.0],
                    uv11,
                    tint_color,
                    [0.0, 1.0, 0.0],
                    light,
                    should_render_backward_up_face(level, fluid_path, pos_up),
                );
            }
        }
        if render_down {
            let uv00 = still_sprite.map_unit_uv(0.0, 0.0);
            let uv10 = still_sprite.map_unit_uv(1.0, 0.0);
            let uv11 = still_sprite.map_unit_uv(1.0, 1.0);
            let uv01 = still_sprite.map_unit_uv(0.0, 1.0);
            add_face(
                vertices,
                indices,
                index_offset,
                [x + 0.0, y + bottom_offs, z + 0.0],
                uv00,
                [x + 1.0, y + bottom_offs, z + 0.0],
                uv10,
                [x + 1.0, y + bottom_offs, z + 1.0],
                uv11,
                [x + 0.0, y + bottom_offs, z + 1.0],
                uv01,
                tint_color,
                [0.0, -1.0, 0.0],
                below_light,
                false,
            );
        }
        for direction in [
            el_math::Direction::North,
            el_math::Direction::South,
            el_math::Direction::West,
            el_math::Direction::East,
        ] {
            let (hh0, hh1, x0, z0, x1, z1, render_condition, face_state) = match direction {
                el_math::Direction::North => (
                    h_nw,
                    h_ne,
                    x,
                    z + 0.001,
                    x + 1.0,
                    z + 0.001,
                    render_north,
                    &block_state_north,
                ),
                el_math::Direction::South => (
                    h_se,
                    h_sw,
                    x + 1.0,
                    z + 1.0 - 0.001,
                    x,
                    z + 1.0 - 0.001,
                    render_south,
                    &block_state_south,
                ),
                el_math::Direction::West => (
                    h_sw,
                    h_nw,
                    x + 0.001,
                    z + 1.0,
                    x + 0.001,
                    z,
                    render_west,
                    &block_state_west,
                ),
                el_math::Direction::East => (
                    h_ne,
                    h_se,
                    x + 1.0 - 0.001,
                    z,
                    x + 1.0 - 0.001,
                    z + 1.0,
                    render_east,
                    &block_state_east,
                ),
                _ => {
                    debug_assert!(false, "fluid side faces are horizontal-only");
                    continue;
                }
            };
            if render_condition
                && !is_face_occluded_by_neighbor(direction, hh0.max(hh1), face_state)
            {
                let (sprite, is_overlay) = match overlay_sprite {
                    Some(overlay)
                        if java_26_2_vanilla_block_fluid_support_properties(face_state)
                            .is_some_and(|properties| properties.fluid_side_overlay) =>
                    {
                        (overlay, true)
                    }
                    _ => (flow_sprite, false),
                };
                let uv0 = sprite.map_unit_uv(0.0, (1.0 - hh0) * 0.5);
                let uv1 = sprite.map_unit_uv(0.5, (1.0 - hh1) * 0.5);
                let uv2 = sprite.map_unit_uv(0.5, 0.5);
                let uv3 = sprite.map_unit_uv(0.0, 0.5);
                let normal = match direction {
                    el_math::Direction::North => [0.0, 0.0, -1.0],
                    el_math::Direction::South => [0.0, 0.0, 1.0],
                    el_math::Direction::West => [-1.0, 0.0, 0.0],
                    el_math::Direction::East => [1.0, 0.0, 0.0],
                    _ => {
                        debug_assert!(false, "fluid side faces are horizontal-only");
                        continue;
                    }
                };
                add_face(
                    vertices,
                    indices,
                    index_offset,
                    [x0, y + hh0, z0],
                    uv0,
                    [x1, y + hh1, z1],
                    uv1,
                    [x1, y + bottom_offs, z1],
                    uv2,
                    [x0, y + bottom_offs, z0],
                    uv3,
                    tint_color,
                    normal,
                    light,
                    !is_overlay,
                );
            }
        }
    }
}
