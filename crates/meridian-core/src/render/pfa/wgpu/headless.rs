use std::{io::Cursor, path::Path, sync::mpsc};

use png::{BitDepth, ColorType, Encoder};
use pollster::block_on;
use wgpu::Extent3d;

use crate::{error::MeridianError, protocol::ImageOutputFormat, render::shared::ProjectedScene};

use super::{VIEWPORT_FORMAT, renderer::PrimitiveSceneRenderer};

pub struct HeadlessRenderSession {
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture: wgpu::Texture,
    renderer: PrimitiveSceneRenderer,
    width: u32,
    height: u32,
}

impl HeadlessRenderSession {
    pub fn new(width: u32, height: u32) -> Result<Self, MeridianError> {
        let instance = wgpu::Instance::default();
        let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .map_err(|e| MeridianError::Wgpu(format!("request_adapter failed: {e}")))?;
        let required_limits = adapter.limits();
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("MeridianHeadlessDevice"),
            required_features: wgpu::Features::empty(),
            required_limits,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: Default::default(),
        }))
        .map_err(|e| MeridianError::Wgpu(format!("request_device failed: {e}")))?;

        Ok(Self {
            texture: create_headless_target(&device, width, height)?,
            renderer: PrimitiveSceneRenderer::new(&device),
            device,
            queue,
            width,
            height,
        })
    }

    pub fn render(&mut self, scene: &ProjectedScene) {
        self.renderer
            .render(&self.device, &self.queue, &self.texture, scene);
    }

    pub fn render_blocking(&mut self, scene: &ProjectedScene) -> Result<(), MeridianError> {
        self.render(scene);
        self.wait_for_gpu()
    }

    pub fn readback_rgba(&self) -> Result<Vec<u8>, MeridianError> {
        let bytes_per_pixel = 4;
        let unpadded_bytes_per_row = self.width as usize * bytes_per_pixel;
        let padded_bytes_per_row =
            unpadded_bytes_per_row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
        let output_size = padded_bytes_per_row * self.height as usize;

        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianHeadlessReadback"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("MeridianHeadlessCopyEncoder"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row as u32),
                    rows_per_image: Some(self.height),
                },
            },
            Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = output_buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map_err(|e| MeridianError::Wgpu(format!("device poll failed: {e}")))?;
        receiver
            .recv()
            .map_err(|e| MeridianError::Wgpu(format!("map_async recv failed: {e}")))?
            .map_err(|e| MeridianError::Wgpu(format!("map_async failed: {e}")))?;

        let mapped = slice.get_mapped_range();
        let mut rgba = Vec::with_capacity(self.width as usize * self.height as usize * 4);
        for row in 0..self.height as usize {
            let row_start = row * padded_bytes_per_row;
            let row_bytes = &mapped[row_start..row_start + unpadded_bytes_per_row];
            rgba.extend_from_slice(row_bytes);
        }
        drop(mapped);
        output_buffer.unmap();

        Ok(rgba)
    }

    pub fn wait_for_gpu(&self) -> Result<(), MeridianError> {
        self.device
            .poll(wgpu::PollType::wait_indefinitely())
            .map(|_| ())
            .map_err(|e| MeridianError::Wgpu(format!("device poll failed: {e}")))
    }
}

pub fn render_scene_headless_to_rgba(
    width: u32,
    height: u32,
    scene: &ProjectedScene,
) -> Result<Vec<u8>, MeridianError> {
    let mut session = HeadlessRenderSession::new(width, height)?;
    session.render(scene);
    session.readback_rgba()
}

pub fn encode_rgba_to_ppm(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut ppm = Vec::with_capacity((width as usize * height as usize * 3) + 64);
    ppm.extend_from_slice(format!("P6\n{} {}\n255\n", width, height).as_bytes());
    for pixel in rgba.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    ppm
}

pub fn encode_rgba_to_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, MeridianError> {
    let mut out = Vec::new();
    {
        let writer = Cursor::new(&mut out);
        let mut encoder = Encoder::new(writer, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut png_writer = encoder
            .write_header()
            .map_err(|e| MeridianError::Io(std::io::Error::other(e.to_string())))?;
        png_writer
            .write_image_data(rgba)
            .map_err(|e| MeridianError::Io(std::io::Error::other(e.to_string())))?;
    }
    Ok(out)
}

pub fn save_scene_headless(
    width: u32,
    height: u32,
    scene: &ProjectedScene,
    format: ImageOutputFormat,
    output: &Path,
) -> Result<u64, MeridianError> {
    let rgba = render_scene_headless_to_rgba(width, height, scene)?;
    let bytes = match format {
        ImageOutputFormat::Ppm => encode_rgba_to_ppm(width, height, &rgba),
        ImageOutputFormat::Png => encode_rgba_to_png(width, height, &rgba)?,
        ImageOutputFormat::Rgba => rgba,
    };
    std::fs::write(output, &bytes)?;
    Ok(bytes.len() as u64)
}

fn create_headless_target(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> Result<wgpu::Texture, MeridianError> {
    validate_headless_target_size(device, width, height)?;
    Ok(device.create_texture(&wgpu::TextureDescriptor {
        label: Some("MeridianHeadlessTarget"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: VIEWPORT_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    }))
}

fn validate_headless_target_size(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> Result<(), MeridianError> {
    let limits = device.limits();
    let max_dimension = limits.max_texture_dimension_2d;
    if width > max_dimension || height > max_dimension {
        return Err(MeridianError::Wgpu(format!(
            "requested headless target {width}x{height} exceeds device 2D texture limit {max_dimension}x{max_dimension}"
        )));
    }
    Ok(())
}
