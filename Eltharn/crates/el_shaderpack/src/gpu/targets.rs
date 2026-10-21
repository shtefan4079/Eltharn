use std::collections::BTreeMap;
pub(super) struct Target {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
}
pub(super) struct Targets {
    pub input: Target,
    pub output: Target,
    pub depth: Option<Target>,
    pub textures: BTreeMap<String, Target>,
    pub decode: wgpu::BindGroup,
    pub encode: wgpu::BindGroup,
    pub capture: Option<wgpu::BindGroup>,
    pub passes: Vec<wgpu::BindGroup>,
}
fn make_target(
    device: &wgpu::Device,
    label: &str,
    format: wgpu::TextureFormat,
    size: [u32; 2],
    usage: wgpu::TextureUsages,
) -> Target {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    Target { texture, view }
}
fn single_texture_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    label: &str,
    view: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(view),
        }],
    })
}
pub(crate) fn binding_error(pass_id: &str, what: &str, name: &str) -> String {
    format!("pass '{pass_id}': unknown {what} '{name}'")
}
fn resolve<'a>(
    pass_id: &str,
    name: &str,
    input: &'a Target,
    textures: &'a BTreeMap<String, Target>,
) -> Result<&'a wgpu::TextureView, String> {
    if name == "engine.scene_color" {
        Ok(&input.view)
    } else {
        textures
            .get(name)
            .map(|target| &target.view)
            .ok_or_else(|| binding_error(pass_id, "texture", name))
    }
}
pub(super) fn create(
    device: &wgpu::Device,
    pipes: &super::pipelines::Pipelines,
    variant: &crate::model::Variant,
    scene_format: wgpu::TextureFormat,
    size: [u32; 2],
    scene: &wgpu::TextureView,
    world_depth: &wgpu::TextureView,
    samplers: &BTreeMap<String, wgpu::Sampler>,
    params: Option<&wgpu::Buffer>,
    camera: Option<&wgpu::Buffer>,
    environment: Option<&wgpu::Buffer>,
) -> Result<Targets, String> {
    let base = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
    let input = make_target(
        device,
        "pack input",
        wgpu::TextureFormat::Rgba8Unorm,
        size,
        base,
    );
    let output = make_target(
        device,
        "pack output",
        scene_format,
        size,
        base | wgpu::TextureUsages::COPY_SRC,
    );
    let mut textures: BTreeMap<String, Target> = BTreeMap::new();
    for name in &variant.textures {
        textures.insert(
            name.clone(),
            make_target(device, name, wgpu::TextureFormat::Rgba8Unorm, size, base),
        );
    }
    let depth: Option<Target> = pipes.depth_layout.as_ref().map(|_| {
        make_target(
            device,
            "pack depth",
            wgpu::TextureFormat::Depth32Float,
            size,
            base,
        )
    });
    let decode = single_texture_group(device, &pipes.codec_layout, "pack decode", scene);
    let final_view = resolve("final output", &variant.final_output, &input, &textures)?;
    let encode = single_texture_group(device, &pipes.codec_layout, "pack encode", final_view);
    let capture: Option<wgpu::BindGroup> = match (&pipes.depth_layout, &depth) {
        (Some(layout), Some(_)) => Some(single_texture_group(
            device,
            layout,
            "pack capture",
            world_depth,
        )),
        (Some(_), None) | (None, Some(_)) => {
            return Err("depth capture: depth layout/texture mismatch".into());
        }
        (None, None) => None,
    };
    let mut passes: Vec<wgpu::BindGroup> = Vec::with_capacity(variant.passes.len());
    for pass in &variant.passes {
        let input_view = resolve(&pass.id, &pass.input, &input, &textures)?;
        let sampler = samplers
            .get(&pass.sampler)
            .ok_or_else(|| binding_error(&pass.id, "sampler", &pass.sampler))?;
        let mut entries: Vec<wgpu::BindGroupEntry> = Vec::with_capacity(6);
        entries.push(wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(input_view),
        });
        entries.push(wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(sampler),
        });
        if let Some(buf) = params {
            entries.push(wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: buf,
                    offset: 0,
                    size: None,
                }),
            });
        }
        if let Some(d) = depth.as_ref() {
            entries.push(wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&d.view),
            });
        }
        if let Some(buf) = camera {
            entries.push(wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: buf,
                    offset: 0,
                    size: None,
                }),
            });
        }
        if let Some(buf) = environment {
            entries.push(wgpu::BindGroupEntry {
                binding: 5,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: buf,
                    offset: 0,
                    size: None,
                }),
            });
        }
        passes.push(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(pass.id.as_str()),
            layout: &pipes.pack_layout,
            entries: &entries,
        }));
    }
    Ok(Targets {
        input,
        output,
        depth,
        textures,
        decode,
        encode,
        capture,
        passes,
    })
}
