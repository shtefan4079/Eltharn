use super::*;

pub(super) struct ItemDrawBindings<'a> {
    pub base_pipeline: &'a wgpu::RenderPipeline,
    pub foil_pipeline: &'a wgpu::RenderPipeline,
    pub atlas: &'a wgpu::BindGroup,
    pub camera: &'a wgpu::BindGroup,
    pub foil_texture: &'a wgpu::BindGroup,
    pub foil_uniform: &'a wgpu::BindGroup,
}

pub(super) fn draw<'a>(
    pass: &mut CountedRenderPass<'a, '_>,
    vertex: &'a wgpu::Buffer,
    index: &'a wgpu::Buffer,
    index_count: u32,
    has_foil: bool,
    bindings: ItemDrawBindings<'a>,
) {
    pass.set_pipeline(bindings.base_pipeline);
    pass.set_bind_group(0, bindings.camera, &[]);
    pass.set_bind_group(1, bindings.atlas, &[]);
    pass.set_vertex_buffer(0, vertex.slice(..));
    pass.set_index_buffer(index.slice(..), wgpu::IndexFormat::Uint32);
    pass.draw_indexed(0..index_count, 0, 0..1);
    if has_foil {
        pass.set_pipeline(bindings.foil_pipeline);
        pass.set_bind_group(2, bindings.foil_texture, &[]);
        pass.set_bind_group(3, bindings.foil_uniform, &[]);
        pass.draw_indexed(0..index_count, 0, 0..1);
    }
}
