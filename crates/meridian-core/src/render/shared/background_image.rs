use std::borrow::Cow;

use bytemuck::{Pod, Zeroable, cast_slice};
use wgpu::util::DeviceExt;

use crate::render::shared::{
    LoadedProjectorImage, ProjectorBackgroundConfig, ProjectorBackgroundScalingMode,
    ProjectorImageConfig, load_projector_image,
};

const BACKGROUND_SHADER: &str = r#"
struct BackgroundUniform {
    uv_min: vec2<f32>,
    uv_max: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> background_uniform: BackgroundUniform;

@group(0) @binding(1)
var background_texture: texture_2d<f32>;

@group(0) @binding(2)
var background_sampler: sampler;

struct VsOut {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @location(0) unit_position: vec2<f32>,
) -> VsOut {
    var out: VsOut;
    out.position = vec4<f32>(
        unit_position.x * 2.0 - 1.0,
        unit_position.y * 2.0 - 1.0,
        0.0,
        1.0,
    );
    out.uv = mix(background_uniform.uv_min, background_uniform.uv_max, unit_position);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return textureSample(background_texture, background_sampler, in.uv);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct BackgroundUniform {
    uv_min: [f32; 2],
    uv_max: [f32; 2],
}

struct BackgroundTextureState {
    selection: ProjectorBackgroundConfig,
    image_size: (u32, u32),
    _texture: wgpu::Texture,
    bind_group: wgpu::BindGroup,
}

pub struct BackgroundImageRenderer {
    vertex_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
    texture: Option<BackgroundTextureState>,
}

impl BackgroundImageRenderer {
    pub fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MeridianBackgroundQuadVertices"),
            contents: cast_slice(&[
                [0.0_f32, 0.0_f32],
                [1.0, 0.0],
                [1.0, 1.0],
                [0.0, 0.0],
                [1.0, 1.0],
                [0.0, 1.0],
            ]),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianBackgroundUniforms"),
            size: std::mem::size_of::<BackgroundUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MeridianBackgroundBindGroupLayout"),
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
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("MeridianBackgroundPipelineLayout"),
            bind_group_layouts: &[&bind_group_layout],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MeridianBackgroundShader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(BACKGROUND_SHADER)),
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("MeridianBackgroundPipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 2]>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });

        Self {
            vertex_buffer,
            uniform_buffer,
            bind_group_layout,
            pipeline,
            texture: None,
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
        background: &ProjectorBackgroundConfig,
        clear_color: wgpu::Color,
    ) -> bool {
        if !self.ensure_texture(device, queue, background) {
            return false;
        }

        let Some(texture) = &self.texture else {
            return false;
        };
        let uv_rect = background_uv_rect(
            target_size,
            texture.image_size,
            background_scaling_mode(background),
        );
        queue.write_buffer(
            &self.uniform_buffer,
            0,
            cast_slice(&[BackgroundUniform {
                uv_min: [uv_rect[0], uv_rect[1]],
                uv_max: [uv_rect[2], uv_rect[3]],
            }]),
        );

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MeridianBackgroundEncoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MeridianBackgroundPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear_color),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &texture.bind_group, &[]);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..6, 0..1);
        }
        queue.submit(Some(encoder.finish()));
        true
    }

    fn ensure_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        background: &ProjectorBackgroundConfig,
    ) -> bool {
        if matches!(background, ProjectorBackgroundConfig::None) {
            self.texture = None;
            return false;
        }
        if self
            .texture
            .as_ref()
            .is_some_and(|state| &state.selection == background)
        {
            return true;
        }

        let loaded = match background {
            ProjectorBackgroundConfig::None => return false,
            ProjectorBackgroundConfig::PngFile { path, .. } => {
                load_projector_image(&ProjectorImageConfig::PngFile { path: path.clone() }, &[])
            }
        };
        let loaded = match loaded {
            Ok(loaded) => loaded,
            Err(error) => {
                eprintln!("failed to load projector background {background:?}: {error}");
                self.texture = None;
                return false;
            }
        };
        self.texture = Some(create_texture_state(
            device,
            queue,
            &self.bind_group_layout,
            &self.uniform_buffer,
            background.clone(),
            loaded,
        ));
        true
    }
}

fn create_texture_state(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    uniform_buffer: &wgpu::Buffer,
    selection: ProjectorBackgroundConfig,
    loaded: LoadedProjectorImage,
) -> BackgroundTextureState {
    let size = wgpu::Extent3d {
        width: loaded.width.max(1),
        height: loaded.height.max(1),
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("MeridianBackgroundTexture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &loaded.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(loaded.width * 4),
            rows_per_image: Some(loaded.height),
        },
        size,
    );
    let texture_view = texture.create_view(&Default::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("MeridianBackgroundSampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("MeridianBackgroundBindGroup"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    BackgroundTextureState {
        selection,
        image_size: (loaded.width, loaded.height),
        _texture: texture,
        bind_group,
    }
}

fn background_scaling_mode(
    background: &ProjectorBackgroundConfig,
) -> ProjectorBackgroundScalingMode {
    match background {
        ProjectorBackgroundConfig::None => ProjectorBackgroundScalingMode::Stretch,
        ProjectorBackgroundConfig::PngFile { scaling, .. } => *scaling,
    }
}

fn background_uv_rect(
    target_size: (u32, u32),
    image_size: (u32, u32),
    scaling_mode: ProjectorBackgroundScalingMode,
) -> [f32; 4] {
    if target_size.0 == 0 || target_size.1 == 0 || image_size.0 == 0 || image_size.1 == 0 {
        return [0.0, 0.0, 1.0, 1.0];
    }

    let target_aspect = target_size.0 as f32 / target_size.1 as f32;
    let image_aspect = image_size.0 as f32 / image_size.1 as f32;
    if image_aspect <= 0.0 || !image_aspect.is_finite() {
        return [0.0, 0.0, 1.0, 1.0];
    }

    match scaling_mode {
        ProjectorBackgroundScalingMode::Stretch => [0.0, 0.0, 1.0, 1.0],
        ProjectorBackgroundScalingMode::Cover => {
            if target_aspect > image_aspect {
                let visible_v = (image_aspect / target_aspect).clamp(0.0, 1.0);
                let margin = (1.0 - visible_v) * 0.5;
                [0.0, margin, 1.0, 1.0 - margin]
            } else {
                let visible_u = (target_aspect / image_aspect).clamp(0.0, 1.0);
                let margin = (1.0 - visible_u) * 0.5;
                [margin, 0.0, 1.0 - margin, 1.0]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::render::shared::ProjectorBackgroundScalingMode;

    use super::background_uv_rect;

    #[test]
    fn cover_crops_horizontally_for_wide_images() {
        let rect = background_uv_rect(
            (100, 100),
            (200, 100),
            ProjectorBackgroundScalingMode::Cover,
        );
        assert_eq!(rect, [0.25, 0.0, 0.75, 1.0]);
    }

    #[test]
    fn cover_crops_vertically_for_tall_images() {
        let rect = background_uv_rect(
            (200, 100),
            (100, 200),
            ProjectorBackgroundScalingMode::Cover,
        );
        assert_eq!(rect, [0.0, 0.375, 1.0, 0.625]);
    }

    #[test]
    fn stretch_uses_full_image() {
        let rect = background_uv_rect(
            (200, 100),
            (100, 200),
            ProjectorBackgroundScalingMode::Stretch,
        );
        assert_eq!(rect, [0.0, 0.0, 1.0, 1.0]);
    }
}
