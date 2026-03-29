use glam::{Mat4, Vec3};

use super::pipeline::{PianoTrailClassicPipelines, Uniforms, VIEWPORT_FORMAT};
use super::streaming::StreamingBufferPool;
use crate::render::{
    SceneLayout,
    piano_trail_classic::model::{PianoTrailClassicQuadInstance, PianoTrailClassicScene},
    shared::{
        BuiltinProjectorImage, LoadedProjectorImage, PianoTrailClassicSceneConfig,
        ProjectorImageConfig, ThreeDSceneConfig, load_projector_image,
    },
};

const OPENGL_TO_WGPU: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.5, 0.0, //
    0.0, 0.0, 0.5, 1.0, //
]);

pub struct PianoTrailClassicRenderer {
    pipelines: PianoTrailClassicPipelines,
    depth: wgpu::Texture,
    note_quads: StreamingBufferPool<PianoTrailClassicQuadInstance>,
    white_key_quads: StreamingBufferPool<PianoTrailClassicQuadInstance>,
    black_key_quads: StreamingBufferPool<PianoTrailClassicQuadInstance>,
    aura_quads: StreamingBufferPool<PianoTrailClassicQuadInstance>,
    aura_texture: Option<AuraTextureState>,
}

struct AuraTextureState {
    selection: ProjectorImageConfig,
    _texture: wgpu::Texture,
    _sampler: wgpu::Sampler,
    bind_group: wgpu::BindGroup,
}

impl PianoTrailClassicRenderer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self {
            pipelines: PianoTrailClassicPipelines::new(device),
            depth: create_depth_texture(device, width, height),
            note_quads: StreamingBufferPool::new("PianoTrailClassicNoteQuads"),
            white_key_quads: StreamingBufferPool::new("PianoTrailClassicWhiteKeyQuads"),
            black_key_quads: StreamingBufferPool::new("PianoTrailClassicBlackKeyQuads"),
            aura_quads: StreamingBufferPool::new("PianoTrailClassicAuraQuads"),
            aura_texture: None,
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::Texture,
        layout: &SceneLayout,
        scene: &PianoTrailClassicScene,
    ) {
        let config = match &layout.scene {
            crate::render::SceneConfig::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => {
                config
            }
            _ => return,
        };
        let mvp = build_mvp(
            config,
            layout.viewport_width as f32 / layout.viewport_height.max(1) as f32,
        );
        queue.write_buffer(
            &self.pipelines.uniform_buffer,
            0,
            bytemuck::bytes_of(&Uniforms {
                mvp: (OPENGL_TO_WGPU * mvp).to_cols_array_2d(),
            }),
        );
        self.ensure_aura_texture(device, queue, &config.aura_image);

        let target_view = target.create_view(&Default::default());
        let depth_view = self.depth.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("PianoTrailClassicEncoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("PianoTrailClassicPass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
            pass.set_bind_group(0, &self.pipelines.color_bind_group, &[]);
            draw_color_chunks(
                device,
                queue,
                &mut self.note_quads,
                &mut pass,
                &self.pipelines.color_always,
                &scene.note_quads,
            );

            let aura_before_keys = (!config.vertical_notes && config.view_offset < 0.0)
                || (config.vertical_notes && config.view_height < 0.025);
            if aura_before_keys {
                if let Some(aura_texture) = &self.aura_texture {
                    pass.set_bind_group(0, &aura_texture.bind_group, &[]);
                }
                draw_aura_chunks(
                    device,
                    queue,
                    &mut self.aura_quads,
                    &mut pass,
                    &self.pipelines.aura_less,
                    &scene.aura_quads,
                );
                pass.set_bind_group(0, &self.pipelines.color_bind_group, &[]);
                draw_color_chunks(
                    device,
                    queue,
                    &mut self.white_key_quads,
                    &mut pass,
                    &self.pipelines.color_less,
                    &scene.white_key_quads,
                );
                draw_color_chunks(
                    device,
                    queue,
                    &mut self.black_key_quads,
                    &mut pass,
                    &self.pipelines.color_less,
                    &scene.black_key_quads,
                );
            } else {
                draw_color_chunks(
                    device,
                    queue,
                    &mut self.white_key_quads,
                    &mut pass,
                    &self.pipelines.color_less,
                    &scene.white_key_quads,
                );
                draw_color_chunks(
                    device,
                    queue,
                    &mut self.black_key_quads,
                    &mut pass,
                    &self.pipelines.color_less,
                    &scene.black_key_quads,
                );
                if let Some(aura_texture) = &self.aura_texture {
                    pass.set_bind_group(0, &aura_texture.bind_group, &[]);
                }
                draw_aura_chunks(
                    device,
                    queue,
                    &mut self.aura_quads,
                    &mut pass,
                    &self.pipelines.aura_less,
                    &scene.aura_quads,
                );
            }
        }
        queue.submit(Some(encoder.finish()));
    }

    fn ensure_aura_texture(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        selection: &ProjectorImageConfig,
    ) {
        if self
            .aura_texture
            .as_ref()
            .is_some_and(|state| &state.selection == selection)
        {
            return;
        }
        let loaded = match load_projector_image(selection, piano_trail_classic_aura_builtins()) {
            Ok(image) => image,
            Err(error) => {
                eprintln!(
                    "failed to load aura image {selection:?}: {error}; falling back to builtin ring"
                );
                load_projector_image(
                    &default_piano_trail_classic_aura_image(),
                    piano_trail_classic_aura_builtins(),
                )
                .expect("builtin ring aura should load")
            }
        };
        self.aura_texture = Some(create_aura_texture_state(
            device,
            queue,
            &self.pipelines,
            selection.clone(),
            loaded,
        ));
    }
}

fn create_aura_texture_state(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipelines: &PianoTrailClassicPipelines,
    selection: ProjectorImageConfig,
    loaded: LoadedProjectorImage,
) -> AuraTextureState {
    let size = wgpu::Extent3d {
        width: loaded.width.max(1),
        height: loaded.height.max(1),
        depth_or_array_layers: 1,
    };
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("PianoTrailClassicAuraTexture"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: VIEWPORT_FORMAT,
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
        label: Some("PianoTrailClassicAuraSampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Linear,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("PianoTrailClassicAuraBindGroup"),
        layout: &pipelines.aura_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: pipelines.uniform_buffer.as_entire_binding(),
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
    AuraTextureState {
        selection,
        _texture: texture,
        _sampler: sampler,
        bind_group,
    }
}

fn piano_trail_classic_aura_builtins() -> &'static [BuiltinProjectorImage] {
    &[BuiltinProjectorImage {
        name: "ring",
        png_bytes: include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/assets/piano_trail_classic/aura_ring.png"
        )),
    }]
}

fn default_piano_trail_classic_aura_image() -> ProjectorImageConfig {
    ProjectorImageConfig::Builtin {
        name: "ring".to_string(),
    }
}

fn draw_color_chunks(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut StreamingBufferPool<PianoTrailClassicQuadInstance>,
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &wgpu::RenderPipeline,
    quads: &[PianoTrailClassicQuadInstance],
) {
    if quads.is_empty() {
        return;
    }
    pass.set_pipeline(pipeline);
    for (chunk_index, chunk) in quads
        .chunks(StreamingBufferPool::<PianoTrailClassicQuadInstance>::max_chunk_len())
        .enumerate()
    {
        buffer.write_chunk(device, queue, chunk_index, chunk);
        pass.set_vertex_buffer(0, buffer.slice(chunk_index));
        pass.draw(0..6, 0..chunk.len() as u32);
    }
}

fn draw_aura_chunks(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut StreamingBufferPool<PianoTrailClassicQuadInstance>,
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &wgpu::RenderPipeline,
    quads: &[PianoTrailClassicQuadInstance],
) {
    if quads.is_empty() {
        return;
    }
    pass.set_pipeline(pipeline);
    for (chunk_index, chunk) in quads
        .chunks(StreamingBufferPool::<PianoTrailClassicQuadInstance>::max_chunk_len())
        .enumerate()
    {
        buffer.write_chunk(device, queue, chunk_index, chunk);
        pass.set_vertex_buffer(0, buffer.slice(chunk_index));
        pass.draw(0..6, 0..chunk.len() as u32);
    }
}

fn build_mvp(config: &PianoTrailClassicSceneConfig, aspect: f32) -> Mat4 {
    let mut model = Mat4::IDENTITY;
    if config.vertical_notes {
        model *= Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    }
    Mat4::perspective_rh_gl(config.fov, aspect.max(0.001), 0.01, 400.0)
        * Mat4::from_rotation_x(config.cam_ang)
        * Mat4::from_rotation_y(config.cam_rot)
        * Mat4::from_rotation_z(config.cam_spin)
        * Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0))
        * Mat4::from_translation(Vec3::new(
            config.view_pan,
            -config.view_height,
            config.view_offset,
        ))
        * model
}

fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("PianoTrailClassicDepthTexture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    })
}

pub fn create_headless_target(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("PianoTrailClassicHeadlessTarget"),
        size: wgpu::Extent3d {
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
