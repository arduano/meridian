use std::{borrow::Cow, io::Cursor, path::Path, sync::mpsc};

use bytemuck::cast_slice;
use png::{BitDepth, ColorType, Encoder};
use pollster::block_on;
use wgpu;
use wgpu::Extent3d;
use wgpu::util::DeviceExt;

use crate::{error::MeridianError, protocol::ImageOutputFormat};

use super::NoteInstance;
use crate::render::shared::{ProjectedScene, SceneLayer, SceneQuad};

const FLAT_SHADER: &str = r#"
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

const NOTE_SHADER: &str = r#"
struct NoteParams {
    piano_height: f32,
    note_pos_factor: f32,
    pad_x: f32,
    pad_y: f32,
};

struct KeyPositions {
    values: array<vec4<f32>, 256>,
};

@group(0) @binding(0)
var<uniform> note_params: NoteParams;

@group(0) @binding(1)
var<uniform> key_positions: KeyPositions;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) size: vec2<f32>,
    @location(3) pad: vec2<f32>,
};

fn mod_add(color: vec4<f32>, add: f32, mul: f32) -> vec4<f32> {
    return vec4<f32>(
        clamp(color.r * mul + add, 0.0, 1.0),
        clamp(color.g * mul + add, 0.0, 1.0),
        clamp(color.b * mul + add, 0.0, 1.0),
        color.a,
    );
}

@vertex
fn vs_main(
    @location(0) unit_position: vec2<f32>,
    @location(1) key: u32,
    @location(2) start: f32,
    @location(3) end: f32,
    @location(4) color: vec4<f32>,
) -> VsOut {
    let key_pos = key_positions.values[key];
    let x1 = key_pos.x;
    let x2 = key_pos.y;
    let y1 = note_params.piano_height + start * note_params.note_pos_factor;
    let y2 = note_params.piano_height + end * note_params.note_pos_factor;

    var out: VsOut;
    let pos = vec2<f32>(mix(x1, x2, unit_position.x), mix(y1, y2, unit_position.y));
    out.position = vec4<f32>(pos.x * 2.0 - 1.0, pos.y * 2.0 - 1.0, 0.5, 1.0);
    out.uv = unit_position;
    out.color = color;
    out.size = vec2<f32>(max(x2 - x1, 1e-6), max(y2 - y1, 1e-6));
    out.pad = vec2<f32>(note_params.pad_x, note_params.pad_y);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    let border_u = clamp(in.pad.x / in.size.x, 0.0, 0.49);
    let border_v = clamp(in.pad.y / in.size.y, 0.0, 0.49);
    let has_inner = in.size.x > in.pad.x * 2.0 && in.size.y > in.pad.y * 2.0;
    let inside_inner =
        has_inner &&
        in.uv.x >= border_u &&
        in.uv.x <= (1.0 - border_u) &&
        in.uv.y >= border_v &&
        in.uv.y <= (1.0 - border_v);

    if inside_inner {
        let inner_u = clamp((in.uv.x - border_u) / max(1.0 - border_u * 2.0, 1e-6), 0.0, 1.0);
        let left = mod_add(in.color, 0.18, 1.0);
        let right = mod_add(in.color, 0.0, 0.55);
        return mix(left, right, inner_u);
    }

    let border_left = mod_add(in.color, 0.0, 0.3);
    let border_right = mod_add(in.color, 0.0, 0.12);
    return mix(border_left, border_right, in.uv.x);
}
"#;

pub const VIEWPORT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
const MIN_STREAMING_INSTANCE_QUADS: usize = 4_096;
const MAX_STREAMING_INSTANCE_QUADS: usize = 1_048_576;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct NoteParamsUniform {
    piano_height: f32,
    note_pos_factor: f32,
    pad_x: f32,
    pad_y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct NoteKeyPositionUniform {
    x1: f32,
    x2: f32,
    _padding: [f32; 2],
}

pub struct PrimitiveSceneRenderer {
    note_pipeline: wgpu::RenderPipeline,
    flat_pipeline: wgpu::RenderPipeline,
    quad_vertex_buffer: wgpu::Buffer,
    note_vertex_buffer: wgpu::Buffer,
    quad_instance_buffer: wgpu::Buffer,
    quad_instance_capacity: usize,
    note_instance_buffer: wgpu::Buffer,
    note_instance_capacity: usize,
    note_params_buffer: wgpu::Buffer,
    note_key_buffer: wgpu::Buffer,
    note_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_extent: Extent3d,
}

impl PrimitiveSceneRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        let flat_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MeridianFlatShader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(FLAT_SHADER)),
        });
        let note_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MeridianNoteShader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(NOTE_SHADER)),
        });

        let flat_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MeridianFlatPipelineLayout"),
            bind_group_layouts: &[],
            immediate_size: 0,
        });
        let note_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("MeridianNoteBindGroupLayout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let note_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MeridianNotePipelineLayout"),
            bind_group_layouts: &[&note_bind_group_layout],
            immediate_size: 0,
        });

        let flat_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MeridianScenePipeline"),
            layout: Some(&flat_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &flat_shader,
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
                module: &flat_shader,
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
            layout: Some(&note_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &note_shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<NoteInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Uint32,
                            2 => Float32,
                            3 => Float32,
                            4 => Unorm8x4
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
                module: &note_shader,
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
        let note_vertices: [[f32; 2]; 6] = [
            [0.0, 0.0],
            [1.0, 0.0],
            [1.0, 1.0],
            [0.0, 0.0],
            [1.0, 1.0],
            [0.0, 1.0],
        ];
        let note_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MeridianNoteVertices"),
            contents: cast_slice(&note_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let quad_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianQuadInstances"),
            size: (MIN_STREAMING_INSTANCE_QUADS * std::mem::size_of::<SceneQuad>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let note_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianNoteInstances"),
            size: (MIN_STREAMING_INSTANCE_QUADS * std::mem::size_of::<NoteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let note_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianNoteParams"),
            size: std::mem::size_of::<NoteParamsUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let note_key_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianNoteKeyPositions"),
            size: (std::mem::size_of::<NoteKeyPositionUniform>() * 256) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let note_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MeridianNoteBindGroup"),
            layout: &note_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: note_params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: note_key_buffer.as_entire_binding(),
                },
            ],
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
            note_vertex_buffer,
            quad_instance_buffer,
            quad_instance_capacity: MIN_STREAMING_INSTANCE_QUADS,
            note_instance_buffer,
            note_instance_capacity: MIN_STREAMING_INSTANCE_QUADS,
            note_params_buffer,
            note_key_buffer,
            note_bind_group,
            depth_texture,
            depth_extent,
        }
    }

    fn ensure_quad_instance_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.quad_instance_capacity {
            return;
        }

        let new_capacity = required.next_power_of_two().max(1024);
        self.quad_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianQuadInstances"),
            size: (new_capacity * std::mem::size_of::<SceneQuad>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.quad_instance_capacity = new_capacity;
    }

    fn ensure_note_instance_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.note_instance_capacity {
            return;
        }

        let new_capacity = required.next_power_of_two().max(1024);
        self.note_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianNoteInstances"),
            size: (new_capacity * std::mem::size_of::<NoteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.note_instance_capacity = new_capacity;
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
        let quad_capacity = [
            SceneLayer::Background,
            SceneLayer::KeyboardDecor,
            SceneLayer::WhiteKeys,
            SceneLayer::BlackKeys,
            SceneLayer::Overlay,
        ]
        .into_iter()
        .map(|layer| scene.layer(layer).len())
        .max()
        .unwrap_or(0)
        .clamp(MIN_STREAMING_INSTANCE_QUADS, MAX_STREAMING_INSTANCE_QUADS);
        let note_capacity = [SceneLayer::WhiteNotes, SceneLayer::BlackNotes]
            .into_iter()
            .map(|layer| scene.note_layer(layer).len())
            .max()
            .unwrap_or(0)
            .clamp(MIN_STREAMING_INSTANCE_QUADS, MAX_STREAMING_INSTANCE_QUADS);
        self.ensure_quad_instance_capacity(device, quad_capacity);
        self.ensure_note_instance_capacity(device, note_capacity);

        let note_params = scene.note_params();
        queue.write_buffer(
            &self.note_params_buffer,
            0,
            cast_slice(&[NoteParamsUniform {
                piano_height: note_params[0],
                note_pos_factor: note_params[1],
                pad_x: note_params[2],
                pad_y: note_params[3],
            }]),
        );
        let key_positions: [NoteKeyPositionUniform; 256] =
            std::array::from_fn(|index| NoteKeyPositionUniform {
                x1: scene.note_key_x()[index][0],
                x2: scene.note_key_x()[index][1],
                _padding: [0.0; 2],
            });
        queue.write_buffer(&self.note_key_buffer, 0, cast_slice(&key_positions));

        let note_layers = if scene.notes_black_first {
            [SceneLayer::BlackNotes, SceneLayer::WhiteNotes]
        } else {
            [SceneLayer::WhiteNotes, SceneLayer::BlackNotes]
        };

        let mut has_color = false;
        for layer in note_layers {
            has_color = self.submit_note_chunks(
                device,
                queue,
                &view,
                Some(&depth_view),
                scene.note_layer(layer),
                has_color,
                "MeridianNoteChunk",
            ) || has_color;
        }

        for layer in [
            SceneLayer::Background,
            SceneLayer::KeyboardDecor,
            SceneLayer::WhiteKeys,
            SceneLayer::BlackKeys,
            SceneLayer::Overlay,
        ] {
            has_color = self.submit_quad_chunks(
                device,
                queue,
                &view,
                None,
                scene.layer(layer),
                has_color,
                "MeridianFlatChunk",
            ) || has_color;
        }

        if !has_color {
            self.clear_target(device, queue, &view);
        }
    }
}

impl PrimitiveSceneRenderer {
    fn submit_note_chunks(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        notes: &[NoteInstance],
        has_existing_color: bool,
        label: &'static str,
    ) -> bool {
        let mut rendered_any = false;
        for (chunk_index, chunk) in notes.chunks(self.note_instance_capacity).enumerate() {
            queue.write_buffer(&self.note_instance_buffer, 0, cast_slice(chunk));

            let mut encoder = device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
            let color_load = if has_existing_color || rendered_any || chunk_index > 0 {
                wgpu::LoadOp::Load
            } else {
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.02,
                    g: 0.05,
                    b: 0.08,
                    a: 1.0,
                })
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: color_load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_view.map(|depth_view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: if has_existing_color || rendered_any || chunk_index > 0 {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(1.0)
                            },
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.note_pipeline);
            pass.set_bind_group(0, &self.note_bind_group, &[]);
            pass.set_vertex_buffer(0, self.note_vertex_buffer.slice(..));
            pass.set_vertex_buffer(
                1,
                self.note_instance_buffer
                    .slice(..(chunk.len() * std::mem::size_of::<NoteInstance>()) as u64),
            );
            pass.draw(0..6, 0..chunk.len() as u32);
            drop(pass);
            queue.submit(Some(encoder.finish()));
            rendered_any = true;
        }

        rendered_any
    }

    fn submit_quad_chunks(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        depth_view: Option<&wgpu::TextureView>,
        quads: &[SceneQuad],
        has_existing_color: bool,
        label: &'static str,
    ) -> bool {
        let mut rendered_any = false;
        for (chunk_index, chunk) in quads.chunks(self.quad_instance_capacity).enumerate() {
            queue.write_buffer(&self.quad_instance_buffer, 0, cast_slice(chunk));

            let mut encoder = device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
            let color_load = if has_existing_color || rendered_any || chunk_index > 0 {
                wgpu::LoadOp::Load
            } else {
                wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.02,
                    g: 0.05,
                    b: 0.08,
                    a: 1.0,
                })
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: color_load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: depth_view.map(|depth_view| {
                    wgpu::RenderPassDepthStencilAttachment {
                        view: depth_view,
                        depth_ops: Some(wgpu::Operations {
                            load: if has_existing_color || rendered_any || chunk_index > 0 {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(1.0)
                            },
                            store: wgpu::StoreOp::Discard,
                        }),
                        stencil_ops: None,
                    }
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.flat_pipeline);
            pass.set_vertex_buffer(0, self.quad_vertex_buffer.slice(..));
            pass.set_vertex_buffer(
                1,
                self.quad_instance_buffer
                    .slice(..(chunk.len() * std::mem::size_of::<SceneQuad>()) as u64),
            );
            pass.draw(0..6, 0..chunk.len() as u32);
            drop(pass);
            queue.submit(Some(encoder.finish()));
            rendered_any = true;
        }

        rendered_any
    }

    fn clear_target(&self, device: &wgpu::Device, queue: &wgpu::Queue, view: &wgpu::TextureView) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MeridianSceneClear"),
        });
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MeridianSceneClear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
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
        let (device, queue) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("MeridianHeadlessDevice"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: Default::default(),
        }))
        .map_err(|e| MeridianError::Wgpu(format!("request_device failed: {e}")))?;

        let texture = create_headless_target(&device, width, height);
        let renderer = PrimitiveSceneRenderer::new(&device);

        Ok(Self {
            device,
            queue,
            texture,
            renderer,
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

fn create_headless_target(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
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
    })
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
