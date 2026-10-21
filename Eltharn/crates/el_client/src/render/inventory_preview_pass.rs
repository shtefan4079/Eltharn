use super::prepare_inventory_preview::PreviewScissor;
pub const PREVIEW_DEPTH_CLEAR: f32 = 0.0;
pub fn upload_preview_mesh<V: bytemuck::Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    vertex_buffer: &mut Option<wgpu::Buffer>,
    vertex_capacity: &mut usize,
    index_buffer: &mut Option<wgpu::Buffer>,
    index_capacity: &mut usize,
    index_count: &mut u32,
    vertices: &[V],
    indices: &[u32],
    label: &str,
) {
    *index_count = indices.len() as u32;
    if *index_count == 0 {
        return;
    }
    let vertex_bytes = bytemuck::cast_slice(vertices);
    if let Some(buffer) = vertex_buffer
        .as_ref()
        .filter(|_| vertex_bytes.len() <= *vertex_capacity)
    {
        queue.write_buffer(buffer, 0, vertex_bytes);
    } else {
        let capacity = vertex_bytes.len().next_power_of_two().max(16384);
        *vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label} Vertex Buffer")),
            size: capacity as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        *vertex_capacity = capacity;
        if let Some(buffer) = vertex_buffer {
            queue.write_buffer(buffer, 0, vertex_bytes);
        }
    }
    let index_bytes = bytemuck::cast_slice(indices);
    if let Some(buffer) = index_buffer
        .as_ref()
        .filter(|_| index_bytes.len() <= *index_capacity)
    {
        queue.write_buffer(buffer, 0, index_bytes);
    } else {
        let capacity = index_bytes.len().next_power_of_two().max(4096);
        *index_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("{label} Index Buffer")),
            size: capacity as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }));
        *index_capacity = capacity;
        if let Some(buffer) = index_buffer {
            queue.write_buffer(buffer, 0, index_bytes);
        }
    }
}
pub struct PreviewBodyDraw<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub index_buffer: &'a wgpu::Buffer,
    pub index_count: u32,
    pub camera_bind_group: &'a wgpu::BindGroup,
    pub texture_bind_group: &'a wgpu::BindGroup,
}
pub struct PreviewArmorDraw<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub index_buffer: &'a wgpu::Buffer,
    pub index_count: u32,
    pub texture_bind_group: &'a wgpu::BindGroup,
    pub has_foil: bool,
    pub decal: bool,
}
pub struct PreviewCapeDraw<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub index_buffer: &'a wgpu::Buffer,
    pub index_count: u32,
    pub texture_bind_group: &'a wgpu::BindGroup,
}
pub struct PreviewItemDraw<'a> {
    pub vertex_buffer: &'a wgpu::Buffer,
    pub index_buffer: &'a wgpu::Buffer,
    pub index_count: u32,
    pub camera_bind_group: &'a wgpu::BindGroup,
    pub texture_bind_group: &'a wgpu::BindGroup,
    pub has_foil: bool,
}
pub struct PreviewPassPipelines<'a> {
    pub preview_pipeline: &'a wgpu::RenderPipeline,
    pub armor_pipeline: &'a wgpu::RenderPipeline,
    pub armor_decal_pipeline: &'a wgpu::RenderPipeline,
    pub item_pipeline: &'a wgpu::RenderPipeline,
    pub item_glint_pipeline: &'a wgpu::RenderPipeline,
    pub cape_pipeline: &'a wgpu::RenderPipeline,
    pub armor_glint_bind_group: &'a wgpu::BindGroup,
    pub armor_glint_uniform_bind_group: &'a wgpu::BindGroup,
    pub item_glint_bind_group: &'a wgpu::BindGroup,
    pub item_glint_uniform_bind_group: &'a wgpu::BindGroup,
}
pub struct PreviewPassDraws<'a> {
    pub body: PreviewBodyDraw<'a>,
    pub armor: &'a [Vec<PreviewArmorDraw<'a>>; 4],
    pub cape: Option<PreviewCapeDraw<'a>>,
    pub items: &'a [Option<PreviewItemDraw<'a>>],
}
pub fn encode_inventory_preview_pass(
    encoder: &mut wgpu::CommandEncoder,
    color_view: &wgpu::TextureView,
    preview_depth_view: &wgpu::TextureView,
    scissor: PreviewScissor,
    pipelines: &PreviewPassPipelines<'_>,
    draws: PreviewPassDraws<'_>,
) {
    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Inventory Player Preview Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: preview_depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(PREVIEW_DEPTH_CLEAR),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    });
    render_pass.set_scissor_rect(scissor.x, scissor.y, scissor.width, scissor.height);
    render_pass.set_pipeline(pipelines.preview_pipeline);
    render_pass.set_bind_group(0, draws.body.camera_bind_group, &[]);
    render_pass.set_bind_group(1, draws.body.texture_bind_group, &[]);
    render_pass.set_vertex_buffer(0, draws.body.vertex_buffer.slice(..));
    render_pass.set_index_buffer(draws.body.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    render_pass.draw_indexed(0..draws.body.index_count, 0, 0..1);
    for armor in crate::render::model_frame::WORLD_ARMOR_DRAW_ORDER
        .iter()
        .flat_map(|&slot| draws.armor[slot].iter())
    {
        render_pass.set_pipeline(if armor.decal {
            pipelines.armor_decal_pipeline
        } else {
            pipelines.armor_pipeline
        });
        render_pass.set_bind_group(0, draws.body.camera_bind_group, &[]);
        render_pass.set_bind_group(1, armor.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, armor.vertex_buffer.slice(..));
        render_pass.set_index_buffer(armor.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..armor.index_count, 0, 0..1);
        if armor.has_foil {
            render_pass.set_pipeline(pipelines.item_glint_pipeline);
            render_pass.set_bind_group(2, pipelines.armor_glint_bind_group, &[]);
            render_pass.set_bind_group(3, pipelines.armor_glint_uniform_bind_group, &[]);
            render_pass.draw_indexed(0..armor.index_count, 0, 0..1);
        }
    }
    if let Some(cape) = draws.cape.as_ref()
        && cape.index_count > 0
    {
        render_pass.set_pipeline(pipelines.cape_pipeline);
        render_pass.set_bind_group(0, draws.body.camera_bind_group, &[]);
        render_pass.set_bind_group(1, cape.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, cape.vertex_buffer.slice(..));
        render_pass.set_index_buffer(cape.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..cape.index_count, 0, 0..1);
    }
    for item in draws.items.iter().flatten() {
        render_pass.set_pipeline(pipelines.item_pipeline);
        render_pass.set_bind_group(0, item.camera_bind_group, &[]);
        render_pass.set_bind_group(1, item.texture_bind_group, &[]);
        render_pass.set_vertex_buffer(0, item.vertex_buffer.slice(..));
        render_pass.set_index_buffer(item.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..item.index_count, 0, 0..1);
        if item.has_foil {
            render_pass.set_pipeline(pipelines.item_glint_pipeline);
            render_pass.set_bind_group(2, pipelines.item_glint_bind_group, &[]);
            render_pass.set_bind_group(3, pipelines.item_glint_uniform_bind_group, &[]);
            render_pass.draw_indexed(0..item.index_count, 0, 0..1);
        }
    }
    drop(render_pass);
}
