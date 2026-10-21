use super::{
    BakedModel, BlockModel, CameraUniform, DynamicGpuMesh, ItemTransform, TextureAtlas, Vertex,
    mesh_from_generated_item_sprite,
};
use crate::modding::resources::ModelAssets;
use crate::render::chunk::ChunkSectionLayer;
use crate::resources::manager::ClientResourceManager;
use el_core::resource_location::ResourceLocation;
use el_mod_content::ContentId;
use el_mod_content::catalog::Catalog;
use el_mod_content::scene_types::{ObjectKind, SceneView};
use el_mod_content::spec::ContentSpec;
use std::collections::BTreeMap;
use wgpu::util::DeviceExt;

const MAX_MODEL_TOTAL_VERTICES: usize = 500_000;
const MAX_MODEL_TOTAL_INDICES: usize = 750_000;
const MAX_SCENE_VERTICES: usize = 125_000;
const MAX_SCENE_INDICES: usize = 750_000;
const MAX_SCENE_OBJECTS: usize = 128 * 16;
const MOD_FULLBRIGHT: [f32; 2] = [15.0 / 16.0, 15.0 / 16.0];

struct ModelMesh {
    vertices: Vec<Vertex>,
    solid: Vec<u32>,
    cutout: Vec<u32>,
    translucent: Vec<u32>,
}

struct TransQuad {
    verts: [Vertex; 4],
    dist2: f64,
}

pub(super) struct ModGpuContent {
    pub(super) atlas: TextureAtlas,
    pub(super) camera_bind_group: wgpu::BindGroup,
    pub(super) solid: DynamicGpuMesh,
    pub(super) cutout: DynamicGpuMesh,
    pub(super) translucent: DynamicGpuMesh,
    pub(super) total_vertices: usize,
    pub(super) solid_draw_indices: usize,
    pub(super) cutout_draw_indices: usize,
    pub(super) translucent_draw_indices: usize,
    camera_buffer: wgpu::Buffer,
    models: BTreeMap<ContentId, ModelMesh>,
    last: Option<(u64, u64, [i64; 3])>,
    last_camera: Option<[u64; 3]>,
    last_has_translucent: bool,
}

fn quad_normal(p0: glam::Vec3, p1: glam::Vec3, p2: glam::Vec3) -> [f32; 3] {
    let n = (p1 - p0).cross(p2 - p0).normalize_or_zero();
    [n.x, n.y, n.z]
}

impl ModGpuContent {
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        camera_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        assets: ModelAssets,
        resources: &ClientResourceManager,
    ) -> Result<Self, String> {
        if assets.textures.is_empty() {
            return Err("mod_content: no textures".to_string());
        }
        let mut atlas = TextureAtlas::from_resource_ids(
            device,
            queue,
            texture_layout,
            resources,
            &assets.textures,
        )
        .map_err(|e| format!("mod atlas: {e:?}"))?;
        atlas.tick_animations(0);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mod_content camera"),
            contents: bytemuck::bytes_of(&CameraUniform::new()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("mod_content camera bind"),
        });
        let mut models: BTreeMap<ContentId, ModelMesh> = BTreeMap::new();
        let mut total_verts: usize = 0;
        let mut total_idx: usize = 0;
        for (id, model) in assets.models.iter() {
            let mut mesh = Self::bake_one(model, &atlas, resources)?;
            let material = assets
                .materials
                .get(id)
                .ok_or("missing validated material")?;
            for vertex in &mut mesh.vertices {
                vertex.color = material.color.map(|c| c as f32 / 255.0);
                vertex.light = [material.light as f32 / 16.0; 2];
            }
            if let Some(layer) = material.layer {
                let indices = mesh
                    .solid
                    .drain(..)
                    .chain(mesh.cutout.drain(..))
                    .chain(mesh.translucent.drain(..))
                    .collect();
                match layer {
                    crate::modding::material::MaterialLayer::Solid => mesh.solid = indices,
                    crate::modding::material::MaterialLayer::Cutout => mesh.cutout = indices,
                    crate::modding::material::MaterialLayer::Translucent => {
                        mesh.translucent = indices
                    }
                }
            }
            total_verts = total_verts
                .checked_add(mesh.vertices.len())
                .ok_or("mod_content: vert overflow")?;
            let idx_len = mesh.solid.len() + mesh.cutout.len() + mesh.translucent.len();
            total_idx = total_idx
                .checked_add(idx_len)
                .ok_or("mod_content: index overflow")?;
            if total_verts > MAX_MODEL_TOTAL_VERTICES || total_idx > MAX_MODEL_TOTAL_INDICES {
                return Err(format!(
                    "mod_content: model budget exceeded at {}",
                    id.as_str()
                ));
            }
            models.insert(id.clone(), mesh);
        }
        Ok(Self {
            atlas,
            camera_bind_group,
            solid: DynamicGpuMesh::default(),
            cutout: DynamicGpuMesh::default(),
            translucent: DynamicGpuMesh::default(),
            total_vertices: 0,
            solid_draw_indices: 0,
            cutout_draw_indices: 0,
            translucent_draw_indices: 0,
            camera_buffer,
            models,
            last: None,
            last_camera: None,
            last_has_translucent: false,
        })
    }

    fn bake_one(
        model: &BlockModel,
        atlas: &TextureAtlas,
        resources: &ClientResourceManager,
    ) -> Result<ModelMesh, String> {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut solid: Vec<u32> = Vec::new();
        let mut cutout: Vec<u32> = Vec::new();
        let mut translucent: Vec<u32> = Vec::new();
        let has_elements = model.elements.as_ref().is_some_and(|e| !e.is_empty());
        if has_elements {
            let baked: BakedModel = BakedModel::bake(model, atlas, 0, 0, false);
            for q in baked.quads.iter() {
                let p = q.vertex_positions;
                let n = quad_normal(p[0], p[1], p[2]);
                let base = vertices.len() as u32;
                for i in 0..4 {
                    vertices.push(Vertex {
                        position: [p[i].x, p[i].y, p[i].z],
                        tex_coords: q.vertex_uvs[i],
                        normal: n,
                        color: [1.0; 4],
                        light: MOD_FULLBRIGHT,
                    });
                }
                let dst = match q.layer {
                    ChunkSectionLayer::Solid => &mut solid,
                    ChunkSectionLayer::Cutout => &mut cutout,
                    ChunkSectionLayer::Translucent => &mut translucent,
                };
                dst.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        } else {
            for layer_idx in 0..10 {
                let key = format!("#layer{layer_idx}");
                if !model
                    .textures
                    .as_ref()
                    .is_some_and(|m| m.contains_key(&format!("layer{layer_idx}")))
                {
                    continue;
                }
                let tex_str = model.resolve_texture(&key);
                let loc = ResourceLocation::from_string(tex_str);
                let (mut v, idx) = mesh_from_generated_item_sprite(
                    &loc,
                    atlas,
                    resources,
                    ItemTransform::default(),
                    false,
                    [1.0; 4],
                    layer_idx,
                );
                if idx.is_empty() {
                    continue;
                }
                let base = vertices.len() as u32;
                for vertex in &mut v {
                    vertex.light = MOD_FULLBRIGHT;
                }
                vertices.extend_from_slice(&v);
                cutout.extend(idx.iter().map(|i| i + base));
            }
        }
        if solid.is_empty() && cutout.is_empty() && translucent.is_empty() {
            return Err("mod_content: model baked to no indices".to_string());
        }
        Ok(ModelMesh {
            vertices,
            solid,
            cutout,
            translucent,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        catalog: &Catalog,
        scene: &SceneView,
        origin: [i64; 3],
        camera_position: [f64; 3],
        game_tick: u64,
    ) -> Result<(), String> {
        self.atlas.tick_animations(game_tick);
        let epoch = scene.epoch;
        let revision = scene.revision;
        let cam_bits = [
            camera_position[0].to_bits(),
            camera_position[1].to_bits(),
            camera_position[2].to_bits(),
        ];
        if let Some((e, r, o)) = self.last {
            if e == epoch
                && r == revision
                && o == origin
                && (!self.last_has_translucent || self.last_camera == Some(cam_bits))
            {
                return Ok(());
            }
        }
        if scene
            .actors
            .iter()
            .map(|actor| actor.objects.len())
            .sum::<usize>()
            > MAX_SCENE_OBJECTS
        {
            return Err("mod_content: too many objects".to_string());
        }
        let mut solid_v: Vec<Vertex> = Vec::new();
        let mut solid_i: Vec<u32> = Vec::new();
        let mut cutout_v: Vec<Vertex> = Vec::new();
        let mut cutout_i: Vec<u32> = Vec::new();
        let mut trans_quads: Vec<TransQuad> = Vec::new();
        for obj in scene.actors.iter().flat_map(|actor| &actor.objects) {
            let kind = match obj.object.kind {
                ObjectKind::Block => el_mod_content::spec::Kind::Block,
                ObjectKind::Entity => el_mod_content::spec::Kind::Entity,
                ObjectKind::BlockEntity => el_mod_content::spec::Kind::BlockEntity,
            };
            let spec = catalog.get(kind, &obj.content)?;
            let (model_id, dims): (ContentId, Option<[u16; 2]>) = match (&obj.object.kind, spec) {
                (
                    ObjectKind::Block,
                    ContentSpec::Block {
                        model,
                        state_models,
                        ..
                    },
                ) => {
                    let k = catalog.block_state_key(&obj.content, &obj.properties)?;
                    (state_models.get(&k).unwrap_or(model).clone(), None)
                }
                (
                    ObjectKind::Entity,
                    ContentSpec::Entity {
                        model,
                        dimensions_milli,
                        ..
                    },
                ) => (model.clone(), Some(*dimensions_milli)),
                (ObjectKind::BlockEntity, ContentSpec::BlockEntity { model, .. }) => {
                    (model.clone(), None)
                }
                _ => return Err("mod_content: kind/spec mismatch".to_string()),
            };
            let mesh = self
                .models
                .get(&model_id)
                .ok_or_else(|| format!("mod_content: unknown model {}", model_id.as_str()))?;
            let base = [
                (obj.position_milli[0] - origin[0] * 1000) as f32 / 1000.0,
                (obj.position_milli[1] - origin[1] * 1000) as f32 / 1000.0,
                (obj.position_milli[2] - origin[2] * 1000) as f32 / 1000.0,
            ];
            if let Some(d) = dims {
                let w = d[0] as f32 / 1000.0;
                let h = d[1] as f32 / 1000.0;
                let rx = (obj.rotation_mdeg[0] as f32 / 1000.0).to_radians();
                let ry = (obj.rotation_mdeg[1] as f32 / 1000.0).to_radians();
                let rz = (obj.rotation_mdeg[2] as f32 / 1000.0).to_radians();
                let rot = glam::Mat3::from_rotation_z(rz)
                    * glam::Mat3::from_rotation_y(ry)
                    * glam::Mat3::from_rotation_x(rx);
                let scale = glam::Mat3::from_cols(
                    glam::Vec3::new(w, 0.0, 0.0),
                    glam::Vec3::new(0.0, h, 0.0),
                    glam::Vec3::new(0.0, 0.0, w),
                );
                let linear = rot * scale;
                let nmat = linear.inverse().transpose();
                let wb = glam::Vec3::from(base);
                let mut t: Vec<Vertex> = Vec::with_capacity(mesh.vertices.len());
                for v in mesh.vertices.iter() {
                    let p = glam::Vec3::from(v.position);
                    let local = glam::Vec3::new(p.x - 0.5, p.y, p.z - 0.5);
                    let wp = linear * local + wb;
                    let n = (nmat * glam::Vec3::from(v.normal)).normalize_or_zero();
                    t.push(Vertex {
                        position: [wp.x, wp.y, wp.z],
                        tex_coords: v.tex_coords,
                        normal: [n.x, n.y, n.z],
                        color: v.color,
                        light: v.light,
                    });
                }
                Self::append_object(
                    mesh,
                    &t,
                    &mut solid_v,
                    &mut solid_i,
                    &mut cutout_v,
                    &mut cutout_i,
                    &mut trans_quads,
                    origin,
                    camera_position,
                )?;
            } else {
                let mut t: Vec<Vertex> = Vec::with_capacity(mesh.vertices.len());
                for v in mesh.vertices.iter() {
                    t.push(Vertex {
                        position: [
                            v.position[0] + base[0],
                            v.position[1] + base[1],
                            v.position[2] + base[2],
                        ],
                        tex_coords: v.tex_coords,
                        normal: v.normal,
                        color: v.color,
                        light: v.light,
                    });
                }
                Self::append_object(
                    mesh,
                    &t,
                    &mut solid_v,
                    &mut solid_i,
                    &mut cutout_v,
                    &mut cutout_i,
                    &mut trans_quads,
                    origin,
                    camera_position,
                )?;
            }
        }
        trans_quads.sort_by(|a, b| b.dist2.total_cmp(&a.dist2));
        let mut trans_v: Vec<Vertex> = Vec::new();
        let mut trans_i: Vec<u32> = Vec::new();
        for q in trans_quads.iter() {
            let base = trans_v.len() as u32;
            trans_v.extend_from_slice(&q.verts);
            trans_i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let total_v = solid_v.len() + cutout_v.len() + trans_v.len();
        let total_i = solid_i.len() + cutout_i.len() + trans_i.len();
        if total_v > MAX_SCENE_VERTICES || total_i > MAX_SCENE_INDICES {
            return Err("mod_content: scene budget exceeded".to_string());
        }
        self.solid
            .update_from_parts(device, queue, &solid_v, &solid_i, "mod solid");
        self.cutout
            .update_from_parts(device, queue, &cutout_v, &cutout_i, "mod cutout");
        self.translucent
            .update_from_parts(device, queue, &trans_v, &trans_i, "mod translucent");
        self.total_vertices = total_v;
        self.solid_draw_indices = solid_i.len();
        self.cutout_draw_indices = cutout_i.len();
        self.translucent_draw_indices = trans_i.len();
        self.last = Some((epoch, revision, origin));
        self.last_camera = Some(cam_bits);
        self.last_has_translucent = !trans_i.is_empty();
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn append_object(
        mesh: &ModelMesh,
        t: &[Vertex],
        solid_v: &mut Vec<Vertex>,
        solid_i: &mut Vec<u32>,
        cutout_v: &mut Vec<Vertex>,
        cutout_i: &mut Vec<u32>,
        trans_quads: &mut Vec<TransQuad>,
        origin: [i64; 3],
        camera_position: [f64; 3],
    ) -> Result<(), String> {
        append_flat(
            solid_v,
            solid_i,
            cutout_v,
            cutout_i,
            trans_quads,
            mesh,
            t,
            origin,
            camera_position,
        )
    }

    pub(super) fn update_camera(&self, queue: &wgpu::Queue, uniform: &CameraUniform) {
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(uniform));
    }
}

fn append_flat(
    solid_v: &mut Vec<Vertex>,
    solid_i: &mut Vec<u32>,
    cutout_v: &mut Vec<Vertex>,
    cutout_i: &mut Vec<u32>,
    trans_quads: &mut Vec<TransQuad>,
    mesh: &ModelMesh,
    t: &[Vertex],
    origin: [i64; 3],
    camera_position: [f64; 3],
) -> Result<(), String> {
    for part in 0..3 {
        let idx: &[u32] = match part {
            0 => &mesh.solid,
            1 => &mesh.cutout,
            _ => &mesh.translucent,
        };
        if idx.len() % 6 != 0 {
            return Err("non-quad model indices".into());
        }
        for quad6 in idx.chunks_exact(6) {
            if quad6.iter().any(|&index| index as usize >= t.len()) {
                return Err("model index out of bounds".into());
            }
            if solid_v.len() + cutout_v.len() + trans_quads.len() * 4 + 4 > MAX_SCENE_VERTICES
                || solid_i.len() + cutout_i.len() + trans_quads.len() * 6 + 6 > MAX_SCENE_INDICES
            {
                return Err("mod scene geometry budget exceeded".into());
            }
            let vi = [
                quad6[0] as usize,
                quad6[1] as usize,
                quad6[2] as usize,
                quad6[5] as usize,
            ];
            if part == 2 {
                let qv = [t[vi[0]], t[vi[1]], t[vi[2]], t[vi[3]]];
                let cx =
                    (qv[0].position[0] + qv[1].position[0] + qv[2].position[0] + qv[3].position[0])
                        * 0.25;
                let cy =
                    (qv[0].position[1] + qv[1].position[1] + qv[2].position[1] + qv[3].position[1])
                        * 0.25;
                let cz =
                    (qv[0].position[2] + qv[1].position[2] + qv[2].position[2] + qv[3].position[2])
                        * 0.25;
                let wx = origin[0] as f64 + cx as f64;
                let wy = origin[1] as f64 + cy as f64;
                let wz = origin[2] as f64 + cz as f64;
                let dx = wx - camera_position[0];
                let dy = wy - camera_position[1];
                let dz = wz - camera_position[2];
                if solid_v.len() + cutout_v.len() + trans_quads.len() * 4 + 4 > MAX_SCENE_VERTICES {
                    return Err("mod_content: scene budget exceeded".to_string());
                }
                trans_quads.push(TransQuad {
                    verts: qv,
                    dist2: dx * dx + dy * dy + dz * dz,
                });
            } else if part == 0 {
                if solid_v.len() + 4 > MAX_SCENE_VERTICES || solid_i.len() + 6 > MAX_SCENE_INDICES {
                    return Err("mod_content: scene budget exceeded".to_string());
                }
                let base = solid_v.len() as u32;
                solid_v.extend_from_slice(&[t[vi[0]], t[vi[1]], t[vi[2]], t[vi[3]]]);
                solid_i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            } else {
                if cutout_v.len() + 4 > MAX_SCENE_VERTICES || cutout_i.len() + 6 > MAX_SCENE_INDICES
                {
                    return Err("mod_content: scene budget exceeded".to_string());
                }
                let base = cutout_v.len() as u32;
                cutout_v.extend_from_slice(&[t[vi[0]], t[vi[1]], t[vi[2]], t[vi[3]]]);
                cutout_i.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
        }
        if idx.len() % 6 != 0 && part != 1 {
            return Err("mod_content: non-quad layer indices".to_string());
        }
    }
    if mesh.cutout.len() % 3 != 0 {
        return Err("mod_content: bad cutout index count".to_string());
    }
    Ok(())
}
