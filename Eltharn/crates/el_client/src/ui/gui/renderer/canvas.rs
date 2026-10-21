use super::*;
use el_mod_content::scene_types::{SceneView, canvas::CanvasView};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Default)]
pub(super) struct CanvasTextures {
    epoch: u64,
    entries: BTreeMap<(String, u32), CanvasTexture>,
}
struct CanvasTexture {
    slot: usize,
    texture: wgpu::Texture,
    width: u8,
    height: u8,
    revision: u64,
}
fn allocate_slot(
    slots: &mut Vec<Option<(TextureView, BindGroup)>>,
    value: (TextureView, BindGroup),
) -> usize {
    if let Some(index) = slots.iter().position(Option::is_none) {
        slots[index] = Some(value);
        index
    } else {
        let index = slots.len();
        slots.push(Some(value));
        index
    }
}
fn texture(
    device: &Device,
    queue: &Queue,
    layout: &wgpu::BindGroupLayout,
    canvas: &CanvasView,
    slots: &mut Vec<Option<(TextureView, BindGroup)>>,
) -> Result<CanvasTexture, String> {
    let bytes = canvas.pixels()?;
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Mod canvas"),
        size: wgpu::Extent3d {
            width: canvas.width.into(),
            height: canvas.height.into(),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    upload(queue, &texture, canvas, &bytes);
    let view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });
    let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Mod canvas GUI binding"),
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
    });
    let slot = allocate_slot(slots, (view, group));
    Ok(CanvasTexture {
        slot,
        texture,
        width: canvas.width,
        height: canvas.height,
        revision: canvas.revision,
    })
}
fn upload(queue: &Queue, texture: &wgpu::Texture, canvas: &CanvasView, bytes: &[u8]) {
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytes,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(u32::from(canvas.width) * 4),
            rows_per_image: Some(canvas.height.into()),
        },
        wgpu::Extent3d {
            width: canvas.width.into(),
            height: canvas.height.into(),
            depth_or_array_layers: 1,
        },
    );
}
impl CanvasTextures {
    fn sync(
        &mut self,
        device: &Device,
        queue: &Queue,
        layout: &wgpu::BindGroupLayout,
        scene: &SceneView,
        slots: &mut Vec<Option<(TextureView, BindGroup)>>,
    ) -> Result<(), String> {
        if self.epoch != scene.epoch {
            for (_, entry) in std::mem::take(&mut self.entries) {
                slots[entry.slot] = None;
            }
            self.epoch = scene.epoch;
        }
        let mut active = BTreeSet::new();
        for actor in &scene.actors {
            for canvas in &actor.canvases {
                let key = (actor.owner.clone(), canvas.id);
                active.insert(key.clone());
                let recreate = self.entries.get(&key).is_some_and(|entry| {
                    entry.width != canvas.width || entry.height != canvas.height
                });
                if recreate {
                    let old = self.entries.remove(&key).unwrap();
                    slots[old.slot] = None;
                }
                if let Some(entry) = self.entries.get_mut(&key) {
                    if entry.revision != canvas.revision {
                        let bytes = canvas.pixels()?;
                        upload(queue, &entry.texture, canvas, &bytes);
                        entry.revision = canvas.revision;
                    }
                } else {
                    self.entries
                        .insert(key, texture(device, queue, layout, canvas, slots)?);
                }
            }
        }
        self.entries.retain(|key, entry| {
            if active.contains(key) {
                true
            } else {
                slots[entry.slot] = None;
                false
            }
        });
        Ok(())
    }
}
impl GuiRenderer {
    pub(crate) fn sync_mod_canvases(
        &mut self,
        device: &Device,
        queue: &Queue,
        layout: &wgpu::BindGroupLayout,
        scene: &SceneView,
    ) -> Result<(), String> {
        self.mod_canvases
            .sync(device, queue, layout, scene, &mut self.custom_textures)
    }
    pub(crate) fn draw_mod_canvas(
        &mut self,
        owner: &str,
        id: u32,
        rect: [i32; 4],
    ) -> Result<(), String> {
        let slot = self
            .mod_canvases
            .entries
            .get(&(owner.into(), id))
            .ok_or("mod canvas GPU slot missing")?
            .slot;
        let [x, y, w, h] = rect;
        self.draw_custom_texture(x as f32, y as f32, w as f32, h as f32, slot, [1.0; 4]);
        Ok(())
    }
}
