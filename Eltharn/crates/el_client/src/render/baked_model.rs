use crate::render::chunk::ChunkSectionLayer;
use crate::render::face_bakery::FaceBakery;
use crate::render::model::BlockModel;
use crate::render::texture::{SpriteTransparency, SpriteUvRect, TextureAtlas};
use el_core::resource_location::ResourceLocation;
use el_math::Direction;
use glam::{Mat4, Vec3};
pub const BLOCK_MODEL_UV_INSET_TEXELS: f32 = 0.0;
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub color: [f32; 4],
    pub light: [f32; 2],
}
#[derive(Clone)]
pub struct BakedModel {
    pub quads: Vec<BakedQuad>,
    pub weighted_groups: Vec<BakedModelGroup>,
    pub weighted_group_random_mode: WeightedGroupRandomMode,
    pub particle_sprite: Option<SpriteUvRect>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightedGroupRandomMode {
    Sequential,
    SharedMultipartSeed,
}
#[derive(Clone)]
pub struct WeightedBakedModel {
    pub weight: u32,
    pub model: BakedModel,
}
#[derive(Clone)]
pub enum BakedModelGroup {
    Single(BakedModel),
    Weighted(Vec<WeightedBakedModel>),
}
#[derive(Clone)]
pub struct BakedQuad {
    pub vertex_positions: [glam::Vec3; 4],
    pub model_uvs: [[f32; 2]; 4],
    pub vertex_uvs: [[f32; 2]; 4],
    pub direction: Direction,
    pub cullface: Option<String>,
    pub tint_index: Option<i32>,
    pub layer: ChunkSectionLayer,
    pub shade: bool,
    pub light_emission: u8,
    pub ambient_occlusion: bool,
}
impl BakedModel {
    pub fn empty() -> Self {
        Self {
            quads: Vec::new(),
            weighted_groups: Vec::new(),
            weighted_group_random_mode: WeightedGroupRandomMode::Sequential,
            particle_sprite: None,
        }
    }
    pub fn has_translucent_material(&self) -> bool {
        self.quads.iter().any(|quad| quad.layer.translucent())
            || self.weighted_groups.iter().any(|group| match group {
                BakedModelGroup::Single(model) => model.has_translucent_material(),
                BakedModelGroup::Weighted(entries) => entries
                    .iter()
                    .any(|entry| entry.model.has_translucent_material()),
            })
    }
    pub fn quads_for_seed(&self, seed: i64) -> Vec<&BakedQuad> {
        let mut output = Vec::new();
        self.collect_quads_for_seed_into(seed, &mut output);
        output
    }
    pub fn collect_quads_for_seed_into<'a>(&'a self, seed: i64, output: &mut Vec<&'a BakedQuad>) {
        output.clear();
        output.extend(self.quads.iter());
        match self.weighted_group_random_mode {
            WeightedGroupRandomMode::Sequential => {
                let mut random = JavaRandom::new(seed);
                self.collect_weighted_quads(&mut random, output);
            }
            WeightedGroupRandomMode::SharedMultipartSeed => {
                let mut root_random = JavaRandom::new(seed);
                let part_seed = root_random.next_long();
                for group in &self.weighted_groups {
                    let mut part_random = JavaRandom::new(part_seed);
                    Self::collect_model_group(group, &mut part_random, output);
                }
            }
        }
    }
    pub fn particle_sprite(&self) -> Option<SpriteUvRect> {
        self.particle_sprite.or_else(|| {
            self.weighted_groups.iter().find_map(|group| match group {
                BakedModelGroup::Single(model) => model.particle_sprite(),
                BakedModelGroup::Weighted(entries) => entries
                    .first()
                    .and_then(|entry| entry.model.particle_sprite()),
            })
        })
    }
    fn collect_weighted_quads<'a>(
        &'a self,
        random: &mut JavaRandom,
        output: &mut Vec<&'a BakedQuad>,
    ) {
        for group in &self.weighted_groups {
            Self::collect_model_group(group, random, output);
        }
    }
    fn collect_model_group<'a>(
        group: &'a BakedModelGroup,
        random: &mut JavaRandom,
        output: &mut Vec<&'a BakedQuad>,
    ) {
        match group {
            BakedModelGroup::Single(model) => {
                output.extend(model.quads.iter());
                model.collect_weighted_quads(random, output);
            }
            BakedModelGroup::Weighted(entries) => {
                Self::collect_weighted_group(entries, random, output);
            }
        }
    }
    fn collect_weighted_group<'a>(
        group: &'a [WeightedBakedModel],
        random: &mut JavaRandom,
        output: &mut Vec<&'a BakedQuad>,
    ) {
        if group.is_empty() {
            warn_weighted_group_once("Skipping empty weighted block model group");
            return;
        }
        if group.iter().any(|entry| entry.weight == 0) {
            warn_weighted_group_once(
                "Skipping weighted block model group with a zero weight entry",
            );
            return;
        }
        let total = group
            .iter()
            .map(|entry| i64::from(entry.weight))
            .sum::<i64>();
        if total > i64::from(i32::MAX) {
            warn_weighted_group_once(&format!(
                "Skipping weighted block model group whose total weight {total} exceeds {}",
                i32::MAX
            ));
            return;
        }
        let mut selection = random.next_int(total as u32);
        if let Some(entry) = group.iter().find(|entry| {
            if selection < entry.weight {
                true
            } else {
                selection -= entry.weight;
                false
            }
        }) {
            output.extend(entry.model.quads.iter());
            entry.model.collect_weighted_quads(random, output);
        }
    }
    pub fn bake(
        model: &BlockModel,
        atlas: &TextureAtlas,
        x_rot: i32,
        y_rot: i32,
        uvlock: bool,
    ) -> Self {
        let mut quads = Vec::new();
        let block_rotation = blockstate_rotation_matrix(x_rot, y_rot);
        if let Some(elements) = &model.elements {
            for el in elements {
                let from = Vec3::from(el.from);
                let to = Vec3::from(el.to);
                let element_rotation = el
                    .rotation
                    .as_ref()
                    .map(|rot| (Vec3::from(rot.origin) / 16.0, rot.matrix()));
                for (face_name, face) in &el.faces {
                    let force_translucent = model.texture_forces_translucency(&face.texture);
                    let tex_str = model.resolve_texture(&face.texture);
                    let tex_id = block_texture_id(tex_str);
                    let facing = match face_name.as_str() {
                        "down" => Direction::Down,
                        "up" => Direction::Up,
                        "north" => Direction::North,
                        "south" => Direction::South,
                        "west" => Direction::West,
                        "east" => Direction::East,
                        _ => continue,
                    };
                    let raw_uvs = if let Some(uv) = face.uv {
                        uv
                    } else {
                        FaceBakery::default_face_uv(from, to, facing)
                    };
                    let uv_transform = if uvlock {
                        uvlock_transform(block_rotation, facing)
                    } else {
                        Mat4::IDENTITY
                    };
                    let (vertex_positions, mut vertex_uvs, final_direction) = FaceBakery::bake_quad(
                        from,
                        to,
                        raw_uvs,
                        face.rotation.unwrap_or(0),
                        uv_transform,
                        facing,
                        block_rotation,
                        element_rotation,
                    );
                    let model_uvs = vertex_uvs;
                    let sprite_rect = atlas.sprite_rect_or_missing(&tex_id);
                    let sprite_dimensions = atlas.sprite_dimensions_or_missing(&tex_id);
                    let sprite_rect = sprite_rect.inset_by_texels(
                        sprite_dimensions.0,
                        sprite_dimensions.1,
                        BLOCK_MODEL_UV_INSET_TEXELS,
                    );
                    let layer = if force_translucent {
                        ChunkSectionLayer::Translucent
                    } else {
                        match atlas.sprite_transparency_or_missing(&tex_id) {
                            SpriteTransparency::Opaque => ChunkSectionLayer::Solid,
                            SpriteTransparency::Cutout => ChunkSectionLayer::Cutout,
                            SpriteTransparency::Translucent => ChunkSectionLayer::Translucent,
                        }
                    };
                    for uv in &mut vertex_uvs {
                        *uv = sprite_rect.map_unit_uv(uv[0], uv[1]);
                    }
                    let cullface = rotate_cullface(face.cullface.as_deref(), block_rotation);
                    quads.push(BakedQuad {
                        vertex_positions,
                        model_uvs,
                        vertex_uvs,
                        direction: final_direction,
                        cullface,
                        tint_index: face.tintindex,
                        layer,
                        shade: el.shade.unwrap_or(true),
                        light_emission: el.light_emission.min(15),
                        ambient_occlusion: model.ambientocclusion.unwrap_or(true),
                    });
                }
            }
        }
        Self {
            quads,
            weighted_groups: Vec::new(),
            weighted_group_random_mode: WeightedGroupRandomMode::Sequential,
            particle_sprite: particle_sprite(model, atlas),
        }
    }
}
fn particle_sprite(model: &BlockModel, atlas: &TextureAtlas) -> Option<SpriteUvRect> {
    let tex_id = particle_texture_id(model)?;
    Some(atlas.sprite_rect_or_missing(&tex_id))
}
fn particle_texture_id(model: &BlockModel) -> Option<ResourceLocation> {
    let tex_str = model.resolve_texture("#particle");
    if tex_str.starts_with('#') {
        return None;
    }
    Some(block_texture_id(tex_str))
}
fn block_texture_id(texture: &str) -> ResourceLocation {
    let loc = ResourceLocation::from_string(texture);
    let explicit_namespace = texture.contains(':');
    let path = if explicit_namespace || loc.path.contains('/') {
        loc.path
    } else {
        format!("block/{}", loc.path)
    };
    ResourceLocation::new(&loc.namespace, &path)
}
pub(crate) fn blockstate_rotation_matrix(x_rot: i32, y_rot: i32) -> Mat4 {
    if x_rot == 0 && y_rot == 0 {
        return Mat4::IDENTITY;
    }
    match (quadrant_steps(x_rot), quadrant_steps(y_rot)) {
        (Some(x_steps), Some(y_steps)) => exact_rotation_y(y_steps) * exact_rotation_x(x_steps),
        _ => {
            Mat4::from_rotation_y(-(y_rot as f32).to_radians())
                * Mat4::from_rotation_x(-(x_rot as f32).to_radians())
        }
    }
}
fn quadrant_steps(degrees: i32) -> Option<usize> {
    (degrees % 90 == 0).then(|| degrees.rem_euclid(360) as usize / 90)
}
fn exact_rotation_x(steps: usize) -> Mat4 {
    match steps & 3 {
        1 => signed_permutation([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),
        2 => signed_permutation([1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]),
        3 => signed_permutation([1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0]),
        _ => Mat4::IDENTITY,
    }
}
fn exact_rotation_y(steps: usize) -> Mat4 {
    match steps & 3 {
        1 => signed_permutation([0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]),
        2 => signed_permutation([-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]),
        3 => signed_permutation([0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0]),
        _ => Mat4::IDENTITY,
    }
}
fn signed_permutation(x_image: [f32; 3], y_image: [f32; 3], z_image: [f32; 3]) -> Mat4 {
    Mat4::from_cols(
        glam::Vec4::new(x_image[0], x_image[1], x_image[2], 0.0),
        glam::Vec4::new(y_image[0], y_image[1], y_image[2], 0.0),
        glam::Vec4::new(z_image[0], z_image[1], z_image[2], 0.0),
        glam::Vec4::W,
    )
}
fn vanilla_uv_transform_local_to_global(face: Direction) -> Mat4 {
    match face {
        Direction::South => Mat4::IDENTITY,
        Direction::East => exact_rotation_y(3),
        Direction::West => exact_rotation_y(1),
        Direction::North => exact_rotation_y(2),
        Direction::Up => exact_rotation_x(1),
        Direction::Down => exact_rotation_x(3),
    }
}
fn uvlock_transform(block_rotation: Mat4, facing: Direction) -> Mat4 {
    if block_rotation == Mat4::IDENTITY {
        return Mat4::IDENTITY;
    }
    let face_action = block_rotation * vanilla_uv_transform_local_to_global(facing);
    let normal = face_action.transform_vector3(Vec3::Z);
    let new_side = Direction::approximate_nearest(normal.x, normal.y, normal.z);
    let global_to_local = vanilla_uv_transform_local_to_global(new_side).transpose();
    (global_to_local * face_action).transpose()
}
fn rotate_cullface(cullface: Option<&str>, block_rotation: Mat4) -> Option<String> {
    let Some(cullface) = cullface else {
        return None;
    };
    let Some(direction) = Direction::from_name(cullface) else {
        return Some(cullface.to_string());
    };
    if block_rotation == Mat4::IDENTITY {
        return Some(direction.name().to_string());
    }
    let (x, y, z) = direction.step();
    let rotated = block_rotation.transform_vector3(Vec3::new(x as f32, y as f32, z as f32));
    Some(
        Direction::approximate_nearest(rotated.x, rotated.y, rotated.z)
            .name()
            .to_string(),
    )
}
fn warn_weighted_group_once(message: &str) {
    use std::collections::HashSet;
    use std::sync::{Mutex, OnceLock};
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let warned = WARNED.get_or_init(|| Mutex::new(HashSet::new()));
    if let Ok(mut warned) = warned.lock() {
        if warned.insert(message.to_string()) {
            log::warn!("{message}");
        }
    }
}
struct JavaRandom {
    seed: u64,
}
impl JavaRandom {
    const MULTIPLIER: u64 = 0x5deece66d;
    const ADDEND: u64 = 0xb;
    const MASK: u64 = (1u64 << 48) - 1;
    fn new(seed: i64) -> Self {
        Self {
            seed: (seed as u64 ^ Self::MULTIPLIER) & Self::MASK,
        }
    }
    fn next(&mut self, bits: u32) -> u32 {
        self.seed = self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::ADDEND)
            & Self::MASK;
        (self.seed >> (48 - bits)) as u32
    }
    fn next_int(&mut self, bound: u32) -> u32 {
        assert!(bound != 0 && bound <= i32::MAX as u32);
        if bound.is_power_of_two() {
            return ((u64::from(bound) * u64::from(self.next(31))) >> 31) as u32;
        }
        loop {
            let bits = self.next(31);
            let value = bits % bound;
            if (bits.wrapping_sub(value).wrapping_add(bound - 1) as i32) >= 0 {
                return value;
            }
        }
    }
    fn next_long(&mut self) -> i64 {
        let upper = i64::from(self.next(32) as i32);
        let lower = i64::from(self.next(32) as i32);
        upper.wrapping_shl(32).wrapping_add(lower)
    }
}
