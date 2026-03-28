use std::borrow::Cow;

use bytemuck::cast_slice;
use wgpu::{Extent3d, util::DeviceExt};

use crate::render::{SceneQuad, pfa::NoteInstance};

use super::shaders::{FLAT_SHADER, NOTE_SHADER};

pub const VIEWPORT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
pub const MIN_STREAMING_INSTANCE_QUADS: usize = 4_096;
pub const MAX_STREAMING_INSTANCE_QUADS: usize = 1_048_576;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct NoteParamsUniform {
    pub piano_height: f32,
    pub note_pos_factor: f32,
    pub pad_x: f32,
    pub pad_y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct NoteKeyPositionUniform {
    pub x1: f32,
    pub x2: f32,
    pub _padding: [f32; 2],
}

pub(super) struct RendererResources {
    pub note_pipeline: wgpu::RenderPipeline,
    pub flat_pipeline: wgpu::RenderPipeline,
    pub quad_vertex_buffer: wgpu::Buffer,
    pub note_vertex_buffer: wgpu::Buffer,
    pub quad_instance_buffer: wgpu::Buffer,
    pub note_instance_buffer: wgpu::Buffer,
    pub note_params_buffer: wgpu::Buffer,
    pub note_key_buffer: wgpu::Buffer,
    pub note_bind_group: wgpu::BindGroup,
    pub depth_texture: wgpu::Texture,
    pub depth_extent: Extent3d,
}

pub(super) fn create_renderer_resources(device: &wgpu::Device) -> RendererResources {
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
    let flat_pipeline = create_flat_pipeline(device, &flat_shader, &flat_pipeline_layout);
    let note_pipeline = create_note_pipeline(device, &note_shader, &note_pipeline_layout);

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
    let note_vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("MeridianNoteVertices"),
        contents: cast_slice(&quad_vertices),
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

    RendererResources {
        note_pipeline,
        flat_pipeline,
        quad_vertex_buffer,
        note_vertex_buffer,
        quad_instance_buffer,
        note_instance_buffer,
        note_params_buffer,
        note_key_buffer,
        note_bind_group,
        depth_texture,
        depth_extent,
    }
}

pub(super) fn create_depth_texture(device: &wgpu::Device, size: Extent3d) -> wgpu::Texture {
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

pub(super) fn clear_color() -> wgpu::LoadOp<wgpu::Color> {
    wgpu::LoadOp::Clear(wgpu::Color {
        r: 0.02,
        g: 0.05,
        b: 0.08,
        a: 1.0,
    })
}

fn create_flat_pipeline(
    device: &wgpu::Device,
    flat_shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("MeridianScenePipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: flat_shader,
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
            module: flat_shader,
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
    })
}

fn create_note_pipeline(
    device: &wgpu::Device,
    note_shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("MeridianSceneNotePipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: note_shader,
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
                        4 => Unorm8x4,
                        5 => Unorm8x4
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
            module: note_shader,
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
    })
}
