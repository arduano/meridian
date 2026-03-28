use std::{borrow::Cow, io::Cursor, path::Path, sync::mpsc};

use bytemuck::cast_slice;
use png::{BitDepth, ColorType, Encoder};
use pollster::block_on;
use wgpu;
use wgpu::Extent3d;
use wgpu::util::DeviceExt;

use crate::{error::MeridianError, protocol::ImageOutputFormat};

use super::{ProjectedScene, SceneLayer, SceneQuad};

const SHADER: &str = r#"
struct QuadInstance {
    pos0: vec2<f32>,
    pos1: vec2<f32>,
    pos2: vec2<f32>,
    pos3: vec2<f32>,
    color0: vec4<f32>,
    color1: vec4<f32>,
    color2: vec4<f32>,
    color3: vec4<f32>,
};

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn bilerp2(a: vec2<f32>, b: vec2<f32>, c: vec2<f32>, d: vec2<f32>, uv: vec2<f32>) -> vec2<f32> {
    let bottom = mix(a, b, uv.x);
    let top = mix(d, c, uv.x);
    return mix(bottom, top, uv.y);
}

fn bilerp4(a: vec4<f32>, b: vec4<f32>, c: vec4<f32>, d: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    let bottom = mix(a, b, uv.x);
    let top = mix(d, c, uv.x);
    return mix(bottom, top, uv.y);
}

@vertex
fn vs_main(
    @location(0) unit_position: vec2<f32>,
    @location(1) pos0: vec2<f32>,
    @location(2) pos1: vec2<f32>,
    @location(3) pos2: vec2<f32>,
    @location(4) pos3: vec2<f32>,
    @location(5) color0: vec4<f32>,
    @location(6) color1: vec4<f32>,
    @location(7) color2: vec4<f32>,
    @location(8) color3: vec4<f32>,
    @location(9) depth: f32,
) -> VsOut {
    let uv = unit_position;
    let pos = bilerp2(pos0, pos1, pos2, pos3, uv);
    var out: VsOut;
    out.position = vec4<f32>(pos.x * 2.0 - 1.0, pos.y * 2.0 - 1.0, depth, 1.0);
    out.color = bilerp4(color0, color1, color2, color3, uv);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return in.color;
}
"#;

pub const VIEWPORT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct PrimitiveSceneRenderer {
    note_pipeline: wgpu::RenderPipeline,
    flat_pipeline: wgpu::RenderPipeline,
    quad_vertex_buffer: wgpu::Buffer,
    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,
    depth_texture: wgpu::Texture,
    depth_extent: Extent3d,
}

impl PrimitiveSceneRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MeridianSceneShader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(SHADER)),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MeridianScenePipelineLayout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });

        let flat_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MeridianScenePipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<SceneQuad>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x2,
                            2 => Float32x2,
                            3 => Float32x2,
                            4 => Float32x2,
                            5 => Float32x4,
                            6 => Float32x4,
                            7 => Float32x4,
                            8 => Float32x4,
                            9 => Float32
                        ],
                    },
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: VIEWPORT_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        let note_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MeridianSceneNotePipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<SceneQuad>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x2,
                            2 => Float32x2,
                            3 => Float32x2,
                            4 => Float32x2,
                            5 => Float32x4,
                            6 => Float32x4,
                            7 => Float32x4,
                            8 => Float32x4,
                            9 => Float32
                        ],
                    },
                ],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: VIEWPORT_FORMAT,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let quad_vertices: [[f32; 2]; 6] = [
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ];
        let quad_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MeridianQuadVertices"),
            contents: cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianQuadInstances"),
            size: std::mem::size_of::<SceneQuad>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let depth_extent = Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        };
        let depth_texture = create_depth_texture(device, depth_extent);

        Self {
            note_pipeline,
            flat_pipeline,
            quad_vertex_buffer,
            instance_buffer,
            instance_capacity: 1,
            depth_texture,
            depth_extent,
        }
    }

    fn ensure_instance_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.instance_capacity {
            return;
        }

        let new_capacity = required.next_power_of_two().max(1024);
        self.instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianQuadInstances"),
            size: (new_capacity * std::mem::size_of::<SceneQuad>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.instance_capacity = new_capacity;
    }

    fn ensure_depth_texture(&mut self, device: &wgpu::Device, target: &wgpu::Texture) {
        let size = target.size();
        if size == self.depth_extent {
            return;
        }
        self.depth_texture = create_depth_texture(device, size);
        self.depth_extent = size;
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::Texture,
        scene: &ProjectedScene,
    ) {
        let view = target.create_view(&wgpu::TextureViewDescriptor::default());
        self.ensure_depth_texture(device, target);
        let depth_view = self
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MeridianSceneEncoder"),
        });

        self.ensure_instance_capacity(device, scene.total_quads().max(1));

        let mut layer_offsets = [0_u64; 7];
        let mut offset = 0_u64;
        for layer in [
            SceneLayer::Background,
            SceneLayer::WhiteNotes,
            SceneLayer::BlackNotes,
            SceneLayer::KeyboardDecor,
            SceneLayer::WhiteKeys,
            SceneLayer::BlackKeys,
            SceneLayer::Overlay,
        ] {
            layer_offsets[layer as usize] = offset;
            let quads = scene.layer(layer);
            if !quads.is_empty() {
                queue.write_buffer(&self.instance_buffer, offset, cast_slice(quads));
                offset += (quads.len() * std::mem::size_of::<SceneQuad>()) as u64;
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MeridianNotePass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.02,
                            g: 0.05,
                            b: 0.08,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.note_pipeline);
            pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));

            let note_layers = if scene.notes_black_first {
                [SceneLayer::BlackNotes, SceneLayer::WhiteNotes]
            } else {
                [SceneLayer::WhiteNotes, SceneLayer::BlackNotes]
            };

            for layer in note_layers {
                let quads = scene.layer(layer);
                if quads.is_empty() {
                    continue;
                }
                let bytes = (quads.len() * std::mem::size_of::<SceneQuad>()) as u64;
                pass.set_vertex_buffer(
                    1,
                    self.instance_buffer.slice(
                        layer_offsets[layer as usize]..layer_offsets[layer as usize] + bytes,
                    ),
                );
                pass.draw(0..6, 0..quads.len() as u32);
            }
        }

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MeridianFlatPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.flat_pipeline);
            pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));

            for layer in [
                SceneLayer::Background,
                SceneLayer::KeyboardDecor,
                SceneLayer::WhiteKeys,
                SceneLayer::BlackKeys,
                SceneLayer::Overlay,
            ] {
                let quads = scene.layer(layer);
                if quads.is_empty() {
                    continue;
                }
                let bytes = (quads.len() * std::mem::size_of::<SceneQuad>()) as u64;
                pass.set_vertex_buffer(
                    1,
                    self.instance_buffer.slice(
                        layer_offsets[layer as usize]..layer_offsets[layer as usize] + bytes,
                    ),
                );
                pass.draw(0..6, 0..quads.len() as u32);
            }
        }

        queue.submit(Some(encoder.finish()));
    }
}

fn create_depth_texture(device: &wgpu::Device, size: Extent3d) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("MeridianSceneDepth"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

pub fn render_scene_headless_to_rgba(
    width: u32,
    height: u32,
    scene: &ProjectedScene,
) -> Result<Vec<u8>, MeridianError> {
    let instance = wgpu::Instance::default();
    let adapter = block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
        .map_err(|e| MeridianError::Wgpu(format!("request_adapter failed: {e}")))?;
    let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("MeridianHeadlessDevice"),
        required_features: wgpu::Features::empty(),
        required_limits: wgpu::Limits::downlevel_defaults(),
        experimental_features: wgpu::ExperimentalFeatures::disabled(),
        memory_hints: wgpu::MemoryHints::Performance,
        trace: Default::default(),
    }))
    .map_err(|e| MeridianError::Wgpu(format!("request_device failed: {e}")))?;

    let texture = device.create_texture(&wgpu::TextureDescriptor {
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
    });

    let mut renderer = PrimitiveSceneRenderer::new(&device);
    renderer.render(&device, &queue, &texture, scene);

    let bytes_per_pixel = 4;
    let unpadded_bytes_per_row = width as usize * bytes_per_pixel;
    let padded_bytes_per_row =
        unpadded_bytes_per_row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize);
    let output_size = padded_bytes_per_row * height as usize;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("MeridianHeadlessReadback"),
        size: output_size as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("MeridianHeadlessCopyEncoder"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row as u32),
                rows_per_image: Some(height),
            },
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));

    let slice = output_buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|e| MeridianError::Wgpu(format!("device poll failed: {e}")))?;
    receiver
        .recv()
        .map_err(|e| MeridianError::Wgpu(format!("map_async recv failed: {e}")))?
        .map_err(|e| MeridianError::Wgpu(format!("map_async failed: {e}")))?;

    let mapped = slice.get_mapped_range();
    let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
    for row in 0..height as usize {
        let row_start = row * padded_bytes_per_row;
        let row_bytes = &mapped[row_start..row_start + unpadded_bytes_per_row];
        rgba.extend_from_slice(row_bytes);
    }
    drop(mapped);
    output_buffer.unmap();

    Ok(rgba)
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
