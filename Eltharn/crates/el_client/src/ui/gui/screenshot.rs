#[cfg(not(target_arch = "wasm32"))]
use image::{ImageFormat, RgbaImage};
#[cfg(not(target_arch = "wasm32"))]
use wgpu::{
    BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, Origin3d, TextureAspect,
};
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChannelOrder {
    Rgba,
    Bgra,
}
#[cfg(not(target_arch = "wasm32"))]
fn channel_order(format: wgpu::TextureFormat) -> Option<ChannelOrder> {
    match format {
        wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Rgba8UnormSrgb => {
            Some(ChannelOrder::Rgba)
        }
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb => {
            Some(ChannelOrder::Bgra)
        }
        _ => None,
    }
}
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ReadbackLayout {
    unpadded_bytes_per_row: u32,
    padded_bytes_per_row: u32,
    buffer_size: wgpu::BufferAddress,
}
#[cfg(not(target_arch = "wasm32"))]
fn readback_layout(width: u32, height: u32) -> Option<ReadbackLayout> {
    if width == 0 || height == 0 {
        return None;
    }
    let unpadded_bytes_per_row = width.checked_mul(4)?;
    let alignment = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let remainder = unpadded_bytes_per_row % alignment;
    let padding = if remainder == 0 {
        0
    } else {
        alignment - remainder
    };
    let padded_bytes_per_row = unpadded_bytes_per_row.checked_add(padding)?;
    let buffer_size = wgpu::BufferAddress::from(padded_bytes_per_row)
        .checked_mul(wgpu::BufferAddress::from(height))?;
    Some(ReadbackLayout {
        unpadded_bytes_per_row,
        padded_bytes_per_row,
        buffer_size,
    })
}
#[cfg(not(target_arch = "wasm32"))]
fn rgba_pixels_from_readback(
    data: &[u8],
    width: u32,
    height: u32,
    bytes_per_row: u32,
    order: ChannelOrder,
) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let packed_row_bytes = usize::try_from(width.checked_mul(4)?).ok()?;
    let row_stride = usize::try_from(bytes_per_row).ok()?;
    if row_stride < packed_row_bytes {
        return None;
    }
    let row_count = usize::try_from(height).ok()?;
    let required_input_len = row_stride.checked_mul(row_count)?;
    if data.len() < required_input_len {
        return None;
    }
    let output_len = packed_row_bytes.checked_mul(row_count)?;
    let mut pixels = Vec::new();
    pixels.try_reserve_exact(output_len).ok()?;
    for y in 0..row_count {
        let row_start = row_stride.checked_mul(y)?;
        let row_end = row_start.checked_add(packed_row_bytes)?;
        let row = data.get(row_start..row_end)?;
        for pixel in row.chunks_exact(4) {
            match order {
                ChannelOrder::Rgba => pixels.extend_from_slice(pixel),
                ChannelOrder::Bgra => {
                    pixels.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]])
                }
            }
        }
    }
    Some(pixels)
}
#[cfg(not(target_arch = "wasm32"))]
pub fn capture_screenshot(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
    output_path: &str,
) -> bool {
    let Some(layout) = readback_layout(width, height) else {
        log::warn!("Screenshot skipped: invalid dimensions {width}x{height}");
        return false;
    };
    let format = texture.format();
    let Some(order) = channel_order(format) else {
        log::warn!("Screenshot skipped: unsupported texture format {format:?}");
        return false;
    };
    let extent = texture.size();
    if texture.dimension() != wgpu::TextureDimension::D2
        || texture.sample_count() != 1
        || extent.width != width
        || extent.height != height
        || extent.depth_or_array_layers != 1
    {
        log::warn!(
            "Screenshot skipped: expected a {width}x{height} single-sample 2D texture, got {:?} {:?} samples={}",
            extent,
            texture.dimension(),
            texture.sample_count()
        );
        return false;
    }
    if !texture.usage().contains(wgpu::TextureUsages::COPY_SRC) {
        log::warn!("Screenshot skipped: source texture lacks COPY_SRC usage");
        return false;
    }
    let output_buffer = device.create_buffer(&BufferDescriptor {
        label: Some("Screenshot Buffer"),
        size: layout.buffer_size,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
        label: Some("Screenshot Encoder"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(layout.padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(std::iter::once(encoder.finish()));
    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = tx.send(result);
    });
    device.poll(wgpu::Maintain::Wait);
    match rx.recv() {
        Ok(Ok(())) => {
            let data = buffer_slice.get_mapped_range();
            let image_result =
                rgba_pixels_from_readback(&data, width, height, layout.padded_bytes_per_row, order)
                    .and_then(|pixels| RgbaImage::from_raw(width, height, pixels));
            drop(data);
            output_buffer.unmap();
            let Some(img) = image_result else {
                log::warn!("Screenshot skipped: mapped readback data had an invalid layout");
                return false;
            };
            if let Some(parent) = std::path::Path::new(output_path).parent()
                && !parent.as_os_str().is_empty()
                && let Err(error) = std::fs::create_dir_all(parent)
            {
                log::warn!("Could not create screenshot directory: {error}");
                return false;
            }
            if let Err(error) = img.save_with_format(output_path, ImageFormat::Png) {
                log::warn!("Could not save screenshot to {output_path}: {error}");
                false
            } else {
                true
            }
        }
        Ok(Err(error)) => {
            log::warn!("Screenshot skipped: GPU readback mapping failed: {error}");
            false
        }
        Err(error) => {
            log::warn!("Screenshot skipped: GPU readback callback failed: {error}");
            false
        }
    }
}
#[cfg(target_arch = "wasm32")]
pub fn capture_screenshot(
    _device: &wgpu::Device,
    _queue: &wgpu::Queue,
    _texture: &wgpu::Texture,
    _width: u32,
    _height: u32,
    _output_path: &str,
) -> bool {
    log::info!("WASM: Screenshot requested (no-op in browser)");
    false
}
