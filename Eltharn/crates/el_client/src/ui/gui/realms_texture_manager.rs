use crate::ui::gui::renderer::GuiRenderer;
use base64::{Engine as _, prelude::BASE64_STANDARD};
use el_core::ResourceLocation;
use image::RgbaImage;
use std::collections::HashMap;
#[derive(Clone, Debug)]
struct RealmsTexture {
    image: String,
    texture_id: ResourceLocation,
}
#[derive(Clone, Debug, Default)]
pub struct RealmsTextureManager {
    textures: HashMap<String, RealmsTexture>,
}
impl RealmsTextureManager {
    pub fn template_icon_location() -> ResourceLocation {
        ResourceLocation::new("minecraft", "gui/presets/isles")
    }
    pub fn missing_texture_location() -> ResourceLocation {
        ResourceLocation::new("minecraft", "missingno")
    }
    pub fn world_template(
        &mut self,
        renderer: &mut GuiRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        id: &str,
        image: Option<&str>,
    ) -> ResourceLocation {
        match image {
            None => Self::template_icon_location(),
            Some(encoded_image) => self.get_texture(
                renderer,
                device,
                queue,
                texture_bind_group_layout,
                id,
                encoded_image,
            ),
        }
    }
    fn get_texture(
        &mut self,
        renderer: &mut GuiRenderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        id: &str,
        encoded_image: &str,
    ) -> ResourceLocation {
        if let Some(texture) = self.textures.get(id)
            && texture.image == encoded_image
        {
            return texture.texture_id.clone();
        }
        let texture_id = match load_image(encoded_image) {
            Some(image) => {
                let texture_id = ResourceLocation::new("realms", &format!("dynamic/{id}"));
                renderer.register_dynamic_texture_rgba(
                    device,
                    queue,
                    texture_bind_group_layout,
                    &texture_id,
                    &image,
                );
                texture_id
            }
            None => Self::missing_texture_location(),
        };
        self.textures.insert(
            id.to_string(),
            RealmsTexture {
                image: encoded_image.to_string(),
                texture_id: texture_id.clone(),
            },
        );
        texture_id
    }
}
fn load_image(encoded_image: &str) -> Option<RgbaImage> {
    let bytes = match BASE64_STANDARD.decode(encoded_image) {
        Ok(bytes) => bytes,
        Err(err) => {
            log::warn!("Failed to decode Realms world image: {err}");
            return None;
        }
    };
    match image::load_from_memory(&bytes) {
        Ok(image) => Some(image.to_rgba8()),
        Err(err) => {
            log::warn!("Failed to load Realms world image: {err}");
            None
        }
    }
}
