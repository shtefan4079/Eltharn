use super::*;
use el_shaderpack::entities_v4::{EntityDraw, EntityMatrices, EntityReplay, MAX_ENTITY_DRAWS};
pub(in crate::client::state) fn body_cutout_cull(
    kind: Option<VanillaEntityModelKind>,
    baby: bool,
) -> Option<bool> {
    use VanillaEntityModelKind::*;
    match kind? {
        Allay | Piglin | ZombifiedPiglin | Vex | Breeze | SulfurCube | WindCharge
        | BreezeWindCharge | WitherSkull | Trident | ShulkerBullet | EnderDragon => None,
        kind => Some(uses_java_entity_cutout_cull_pipeline(Some(kind), baby)),
    }
}
pub(in crate::client::state) fn entity_cutout_cull(
    entity: &RemoteEntityRenderState,
    baby: bool,
) -> Option<bool> {
    if entity.shared_flag(5)
        || (remote_entity_type_key(&entity.kind) == "armor_stand"
            && entity.armor_stand_client_flags & 0x10 != 0)
    {
        return None;
    }
    body_cutout_cull(vanilla_entity_model_kind(&entity.kind), baby)
}
pub(in crate::client::state) struct EntityPacket {
    pub entity_id: i32,
    pub texture: &'static str,
    pub mesh: FrameMeshSlice,
    pub fallback: bool,
    pub cull: bool,
    pub visible: bool,
    pub casts_shadow: bool,
    pub model: glam::Mat4,
    pub camera_clip: [[f32; 4]; 4],
    pub light_uv: [f32; 2],
}
pub(in crate::client::state) struct EntityFrame {
    pub packets: Vec<EntityPacket>,
    pub ids: HashSet<i32>,
    pub candidates: HashSet<i32>,
    pub shadow: Option<el_shaderpack::shadow_v4::ShadowFrame>,
    pub overflow: bool,
    mesh_bytes: usize,
    pub light_enabled: bool,
}
impl EntityFrame {
    pub fn new_with_light(
        shadow: Option<el_shaderpack::shadow_v4::ShadowFrame>,
        light_enabled: bool,
    ) -> Self {
        Self {
            packets: Vec::new(),
            ids: HashSet::new(),
            candidates: HashSet::new(),
            shadow,
            overflow: false,
            mesh_bytes: 0,
            light_enabled,
        }
    }
    pub fn register_candidate(&mut self, id: i32) {
        if self.candidates.len() == MAX_ENTITY_DRAWS && !self.candidates.contains(&id) {
            self.overflow = true;
            return;
        }
        self.candidates.insert(id);
    }
    pub fn shadow_candidate(&self, id: i32) -> bool {
        !self.overflow && self.shadow.is_some() && self.candidates.contains(&id)
    }
    pub fn reserve_mesh(&mut self, id: i32, vertices: usize, indices: usize) -> bool {
        let bytes = vertices
            .checked_mul(std::mem::size_of::<Vertex>())
            .and_then(|v| indices.checked_mul(4).and_then(|i| v.checked_add(i)));
        let total = bytes.and_then(|n| self.mesh_bytes.checked_add(n));
        if self.overflow
            || total.is_none_or(|n| n > el_shaderpack::entities_v4::MAX_ENTITY_MESH_BYTES)
        {
            self.overflow = true;
            self.ids.insert(id);
            return false;
        }
        self.mesh_bytes = total.unwrap();
        true
    }
    pub fn casts_shadow(
        &self,
        vertices: &[Vertex],
        model: glam::Mat4,
        origin: glam::DVec3,
    ) -> bool {
        let Some(shadow) = &self.shadow else {
            return false;
        };
        if vertices.is_empty() {
            return false;
        }
        let mut min = glam::Vec3::splat(f32::INFINITY);
        let mut max = glam::Vec3::splat(f32::NEG_INFINITY);
        for vertex in vertices {
            let world =
                model.transform_point3(glam::Vec3::from_array(vertex.position)) + origin.as_vec3();
            if !world.is_finite() {
                return false;
            }
            min = min.min(world);
            max = max.max(world);
        }
        shadow.intersects_aabb(min.to_array(), max.to_array())
    }
    pub fn push(&mut self, packet: EntityPacket) {
        if self.ids.contains(&packet.entity_id) {
            return;
        }
        self.ids.insert(packet.entity_id);
        if !packet.model.is_finite()
            || packet.model.determinant() == 0.0
            || !packet.model.inverse().is_finite()
        {
            return;
        }
        if !packet.visible && !packet.casts_shadow {
            return;
        }
        if self.packets.len() == MAX_ENTITY_DRAWS {
            self.overflow = true;
            return;
        }
        self.packets.push(packet);
    }
}
impl ClientRenderer {
    pub(in crate::client::state) fn encode_shader_entities(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        frame: Option<&EntityFrame>,
        origin: glam::DVec3,
        shadow_target: Option<&wgpu::TextureView>,
    ) -> Result<(), String> {
        let Some(frame) = frame else {
            self.shader_entities = None;
            return Ok(());
        };
        if frame.overflow {
            return Err(
                "cutout entity budget exceeded (4096 candidates / 8MiB posed meshes)".into(),
            );
        }
        if self
            .shader_entities
            .as_ref()
            .is_none_or(|r| r.light_enabled() != frame.light_enabled)
        {
            self.shader_entities = Some(EntityReplay::new_with_light(
                device,
                &self.texture_bind_group_layout,
                frame.light_enabled,
            )?);
        }
        let matrices: Vec<_> = frame
            .packets
            .iter()
            .map(|p| {
                let light = frame.shadow.as_ref().map_or(glam::Mat4::IDENTITY, |s| {
                    glam::Mat4::from_cols_array_2d(&s.view_projection)
                });
                EntityMatrices {
                    camera_clip: p.camera_clip,
                    light_clip: (light * glam::Mat4::from_translation(origin.as_vec3()) * p.model)
                        .to_cols_array_2d(),
                    world_normal: p.model.inverse().transpose().to_cols_array_2d(),
                }
            })
            .collect();
        let replay = self.shader_entities.as_mut().unwrap();
        if frame.light_enabled {
            let light: Vec<_> = frame.packets.iter().map(|p| p.light_uv).collect();
            replay.prepare_with_light(queue, &matrices, &light)?;
        } else {
            replay.prepare(queue, &matrices)?;
        }
        let draws: Result<Vec<_>, String> = frame
            .packets
            .iter()
            .map(|p| {
                let arena = if p.fallback {
                    &self.fallback_entity_mesh_arena
                } else {
                    &self.pretranslucent_entity_mesh_arena
                };
                let binding = arena
                    .uploaded()
                    .ok_or("entity replay arena was not uploaded")?
                    .resolve(p.mesh);
                Ok(EntityDraw {
                    vertices: binding.vertex_buffer(),
                    vertex_range: binding.vertex_range(),
                    indices: binding.index_buffer(),
                    index_range: binding.index_range(),
                    index_count: binding.index_count(),
                    texture: self
                        .remote_entity_textures
                        .get(p.texture)
                        .ok_or("entity replay texture disappeared")?,
                    cull: p.cull,
                    geometry: p.visible,
                    shadow: p.casts_shadow,
                })
            })
            .collect();
        let draws = draws?;
        if let Some(shadow_target) = shadow_target {
            replay.encode_shadow(encoder, shadow_target, &draws)?;
        }
        let (normal, albedo) = self
            .shader_geometry
            .as_ref()
            .ok_or("entity geometry target absent")?
            .views();
        let light = self.shader_geometry.as_ref().and_then(|g| g.light_view());
        replay.encode_geometry_with_light(
            encoder,
            normal,
            albedo,
            light,
            &self.depth_texture,
            &draws,
        )
    }
}
