use crate::render::model_frame::{FrameLayerCommand, FrameLayerKind};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarPipelineId {
    BodyTranslucent,
    Armor,
    Item,
    ItemPreview,
    CapeCutoutCull,
    ArmorGlint,
    ItemGlint,
}
#[inline]
pub fn body_pipeline_id() -> AvatarPipelineId {
    AvatarPipelineId::BodyTranslucent
}
#[inline]
pub fn armor_pipeline_id() -> AvatarPipelineId {
    AvatarPipelineId::Armor
}
#[inline]
pub fn item_pipeline_id(custom_head_texture: bool) -> AvatarPipelineId {
    if custom_head_texture {
        AvatarPipelineId::ItemPreview
    } else {
        AvatarPipelineId::Item
    }
}
#[inline]
pub fn cape_pipeline_id() -> AvatarPipelineId {
    AvatarPipelineId::CapeCutoutCull
}
#[inline]
pub fn armor_foil_pipeline_id() -> AvatarPipelineId {
    AvatarPipelineId::ArmorGlint
}
#[inline]
pub fn item_foil_pipeline_id() -> AvatarPipelineId {
    AvatarPipelineId::ItemGlint
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvatarOp {
    DrawBody,
    DrawArmor { slot: u8 },
    DrawArmorFoil { slot: u8 },
    DrawMainItem,
    DrawMainItemFoil,
    DrawOffItem,
    DrawOffItemFoil,
    DrawCape,
}
pub fn plan_avatar_ops(frame: &[FrameLayerCommand]) -> Vec<AvatarOp> {
    let mut ops = Vec::with_capacity(frame.len() * 2);
    for command in frame {
        match command.kind {
            FrameLayerKind::Body => ops.push(AvatarOp::DrawBody),
            FrameLayerKind::Armor(slot) => {
                ops.push(AvatarOp::DrawArmor { slot });
                if command.has_foil {
                    ops.push(AvatarOp::DrawArmorFoil { slot });
                }
            }
            FrameLayerKind::MainItem => {
                ops.push(AvatarOp::DrawMainItem);
                if command.has_foil {
                    ops.push(AvatarOp::DrawMainItemFoil);
                }
            }
            FrameLayerKind::OffItem => {
                ops.push(AvatarOp::DrawOffItem);
                if command.has_foil {
                    ops.push(AvatarOp::DrawOffItemFoil);
                }
            }
            FrameLayerKind::Cape => ops.push(AvatarOp::DrawCape),
        }
    }
    ops
}
pub fn pipeline_for_op(
    op: AvatarOp,
    main_custom_head: bool,
    off_custom_head: bool,
) -> AvatarPipelineId {
    match op {
        AvatarOp::DrawBody => body_pipeline_id(),
        AvatarOp::DrawArmor { .. } => armor_pipeline_id(),
        AvatarOp::DrawArmorFoil { .. } => armor_foil_pipeline_id(),
        AvatarOp::DrawMainItem | AvatarOp::DrawMainItemFoil => {
            if matches!(op, AvatarOp::DrawMainItemFoil) {
                item_foil_pipeline_id()
            } else {
                item_pipeline_id(main_custom_head)
            }
        }
        AvatarOp::DrawOffItem | AvatarOp::DrawOffItemFoil => {
            if matches!(op, AvatarOp::DrawOffItemFoil) {
                item_foil_pipeline_id()
            } else {
                item_pipeline_id(off_custom_head)
            }
        }
        AvatarOp::DrawCape => cape_pipeline_id(),
    }
}
#[inline]
pub fn should_reuse_buffer(prev_capacity_bytes: usize, next_bytes_len: usize) -> bool {
    next_bytes_len <= prev_capacity_bytes
}
pub struct AvatarPassPipelines<'a> {
    pub body_pipeline: &'a wgpu::RenderPipeline,
    pub armor_pipeline: &'a wgpu::RenderPipeline,
    pub armor_decal_pipeline: &'a wgpu::RenderPipeline,
    pub item_pipeline: &'a wgpu::RenderPipeline,
    pub item_preview_pipeline: &'a wgpu::RenderPipeline,
    pub cape_pipeline: &'a wgpu::RenderPipeline,
    pub glint_pipeline: &'a wgpu::RenderPipeline,
    pub armor_glint_bind_group: &'a wgpu::BindGroup,
    pub armor_glint_uniform_bind_group: &'a wgpu::BindGroup,
    pub item_glint_bind_group: &'a wgpu::BindGroup,
    pub item_glint_uniform_bind_group: &'a wgpu::BindGroup,
}
pub struct AvatarMeshDraw<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub vertex_range: std::ops::Range<wgpu::BufferAddress>,
    pub index_buffer: &'a wgpu::Buffer,
    pub index_range: std::ops::Range<wgpu::BufferAddress>,
    pub index_count: u32,
    pub camera_bind_group: &'a wgpu::BindGroup,
    pub texture_bind_group: &'a wgpu::BindGroup,
}
impl<'a> AvatarMeshDraw<'a> {
    pub fn whole_buffers(
        vertex_buffer: &'a wgpu::Buffer,
        index_buffer: &'a wgpu::Buffer,
        index_count: u32,
        camera_bind_group: &'a wgpu::BindGroup,
        texture_bind_group: &'a wgpu::BindGroup,
    ) -> Self {
        Self {
            vertex_range: 0..vertex_buffer.size(),
            index_range: 0..index_buffer.size(),
            vertex_buffer,
            index_buffer,
            index_count,
            camera_bind_group,
            texture_bind_group,
        }
    }
}
pub struct AvatarArmorDraw<'a> {
    pub mesh: AvatarMeshDraw<'a>,
    pub has_foil: bool,
    pub decal: bool,
}
pub struct AvatarItemDraw<'a> {
    pub mesh: AvatarMeshDraw<'a>,
    pub has_foil: bool,
    pub use_preview_pipeline: bool,
}
pub struct AvatarPassDraws<'a> {
    pub body: Option<AvatarMeshDraw<'a>>,
    pub armor: [Vec<AvatarArmorDraw<'a>>; 4],
    pub main_item: Option<AvatarItemDraw<'a>>,
    pub off_item: Option<AvatarItemDraw<'a>>,
    pub cape: Option<AvatarMeshDraw<'a>>,
}
pub trait AvatarPassEncoder<'a> {
    fn avatar_set_pipeline(&mut self, pipeline: &'a wgpu::RenderPipeline);
    fn avatar_set_bind_group(
        &mut self,
        index: u32,
        bind_group: &'a wgpu::BindGroup,
        offsets: &[wgpu::DynamicOffset],
    );
    fn avatar_set_vertex_buffer(&mut self, slot: u32, slice: wgpu::BufferSlice<'a>);
    fn avatar_set_index_buffer(&mut self, slice: wgpu::BufferSlice<'a>, format: wgpu::IndexFormat);
    fn avatar_draw_indexed(
        &mut self,
        indices: std::ops::Range<u32>,
        base_vertex: i32,
        instances: std::ops::Range<u32>,
    );
}
impl<'a> AvatarPassEncoder<'a> for wgpu::RenderPass<'a> {
    fn avatar_set_pipeline(&mut self, pipeline: &'a wgpu::RenderPipeline) {
        self.set_pipeline(pipeline);
    }
    fn avatar_set_bind_group(
        &mut self,
        index: u32,
        bind_group: &'a wgpu::BindGroup,
        offsets: &[wgpu::DynamicOffset],
    ) {
        self.set_bind_group(index, bind_group, offsets);
    }
    fn avatar_set_vertex_buffer(&mut self, slot: u32, slice: wgpu::BufferSlice<'a>) {
        self.set_vertex_buffer(slot, slice);
    }
    fn avatar_set_index_buffer(&mut self, slice: wgpu::BufferSlice<'a>, format: wgpu::IndexFormat) {
        self.set_index_buffer(slice, format);
    }
    fn avatar_draw_indexed(
        &mut self,
        indices: std::ops::Range<u32>,
        base_vertex: i32,
        instances: std::ops::Range<u32>,
    ) {
        self.draw_indexed(indices, base_vertex, instances);
    }
}
pub fn encode_avatar_layers<'a, E: AvatarPassEncoder<'a> + ?Sized>(
    pass: &mut E,
    pipelines: &AvatarPassPipelines<'a>,
    draws: &AvatarPassDraws<'a>,
) {
    if let Some(body) = draws.body.as_ref() {
        pass.avatar_set_pipeline(pipelines.body_pipeline);
        pass.avatar_set_bind_group(0, body.camera_bind_group, &[]);
        pass.avatar_set_bind_group(1, body.texture_bind_group, &[]);
        pass.avatar_set_vertex_buffer(0, body.vertex_buffer.slice(body.vertex_range.clone()));
        pass.avatar_set_index_buffer(
            body.index_buffer.slice(body.index_range.clone()),
            wgpu::IndexFormat::Uint32,
        );
        pass.avatar_draw_indexed(0..body.index_count, 0, 0..1);
    }
    for armor in crate::render::model_frame::WORLD_ARMOR_DRAW_ORDER
        .iter()
        .flat_map(|&slot| draws.armor[slot].iter())
    {
        pass.avatar_set_pipeline(if armor.decal {
            pipelines.armor_decal_pipeline
        } else {
            pipelines.armor_pipeline
        });
        pass.avatar_set_bind_group(0, armor.mesh.camera_bind_group, &[]);
        pass.avatar_set_bind_group(1, armor.mesh.texture_bind_group, &[]);
        pass.avatar_set_vertex_buffer(
            0,
            armor
                .mesh
                .vertex_buffer
                .slice(armor.mesh.vertex_range.clone()),
        );
        pass.avatar_set_index_buffer(
            armor
                .mesh
                .index_buffer
                .slice(armor.mesh.index_range.clone()),
            wgpu::IndexFormat::Uint32,
        );
        pass.avatar_draw_indexed(0..armor.mesh.index_count, 0, 0..1);
        if armor.has_foil {
            pass.avatar_set_pipeline(pipelines.glint_pipeline);
            pass.avatar_set_bind_group(2, pipelines.armor_glint_bind_group, &[]);
            pass.avatar_set_bind_group(3, pipelines.armor_glint_uniform_bind_group, &[]);
            pass.avatar_draw_indexed(0..armor.mesh.index_count, 0, 0..1);
        }
    }
    for item in [draws.main_item.as_ref(), draws.off_item.as_ref()]
        .into_iter()
        .flatten()
    {
        pass.avatar_set_pipeline(if item.use_preview_pipeline {
            pipelines.item_preview_pipeline
        } else {
            pipelines.item_pipeline
        });
        pass.avatar_set_bind_group(0, item.mesh.camera_bind_group, &[]);
        pass.avatar_set_bind_group(1, item.mesh.texture_bind_group, &[]);
        pass.avatar_set_vertex_buffer(
            0,
            item.mesh
                .vertex_buffer
                .slice(item.mesh.vertex_range.clone()),
        );
        pass.avatar_set_index_buffer(
            item.mesh.index_buffer.slice(item.mesh.index_range.clone()),
            wgpu::IndexFormat::Uint32,
        );
        pass.avatar_draw_indexed(0..item.mesh.index_count, 0, 0..1);
        if item.has_foil {
            pass.avatar_set_pipeline(pipelines.glint_pipeline);
            pass.avatar_set_bind_group(2, pipelines.item_glint_bind_group, &[]);
            pass.avatar_set_bind_group(3, pipelines.item_glint_uniform_bind_group, &[]);
            pass.avatar_draw_indexed(0..item.mesh.index_count, 0, 0..1);
        }
    }
    if let Some(cape) = draws.cape.as_ref() {
        pass.avatar_set_pipeline(pipelines.cape_pipeline);
        pass.avatar_set_bind_group(0, cape.camera_bind_group, &[]);
        pass.avatar_set_bind_group(1, cape.texture_bind_group, &[]);
        pass.avatar_set_vertex_buffer(0, cape.vertex_buffer.slice(cape.vertex_range.clone()));
        pass.avatar_set_index_buffer(
            cape.index_buffer.slice(cape.index_range.clone()),
            wgpu::IndexFormat::Uint32,
        );
        pass.avatar_draw_indexed(0..cape.index_count, 0, 0..1);
    }
}
