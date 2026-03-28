use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;

use super::pipeline::{MiditrailPipelines, Uniforms, VIEWPORT_FORMAT};
use crate::render::{
    SceneLayout,
    miditrail::model::{MiditrailAuraVertex, MiditrailScene, MiditrailVertex},
    shared::{MiditrailSceneConfig, ThreeDSceneConfig},
};

const OPENGL_TO_WGPU: Mat4 = Mat4::from_cols_array(&[
    1.0, 0.0, 0.0, 0.0, //
    0.0, 1.0, 0.0, 0.0, //
    0.0, 0.0, 0.5, 0.0, //
    0.0, 0.0, 0.5, 1.0, //
]);

pub struct MiditrailRenderer {
    pipelines: MiditrailPipelines,
    depth: wgpu::Texture,
}

impl MiditrailRenderer {
    pub fn new(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self {
            pipelines: MiditrailPipelines::new(device),
            depth: create_depth_texture(device, width, height),
        }
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::Texture,
        layout: &SceneLayout,
        scene: &MiditrailScene,
    ) {
        let config = match &layout.scene {
            crate::render::SceneConfig::ThreeD(ThreeDSceneConfig::Miditrail(config)) => config,
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

        let target_view = target.create_view(&Default::default());
        let depth_view = self.depth.create_view(&Default::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("MiditrailEncoder"),
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("MiditrailPass"),
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
            pass.set_bind_group(0, &self.pipelines.bind_group, &[]);
            draw_color(
                device,
                &mut pass,
                &self.pipelines.color_always,
                &scene.note_vertices,
            );

            let aura_before_keys = (!config.vertical_notes && config.view_offset < 0.0)
                || (config.vertical_notes && config.view_height < 0.025);
            if aura_before_keys {
                draw_aura(
                    device,
                    &mut pass,
                    &self.pipelines.aura_always,
                    &scene.aura_vertices,
                );
                draw_color(
                    device,
                    &mut pass,
                    &self.pipelines.color_less,
                    &scene.white_key_vertices,
                );
                draw_color(
                    device,
                    &mut pass,
                    &self.pipelines.color_less,
                    &scene.black_key_vertices,
                );
            } else {
                draw_color(
                    device,
                    &mut pass,
                    &self.pipelines.color_less,
                    &scene.white_key_vertices,
                );
                draw_color(
                    device,
                    &mut pass,
                    &self.pipelines.color_less,
                    &scene.black_key_vertices,
                );
                draw_aura(
                    device,
                    &mut pass,
                    &self.pipelines.aura_always,
                    &scene.aura_vertices,
                );
            }
        }
        queue.submit(Some(encoder.finish()));
    }
}

fn draw_color(
    device: &wgpu::Device,
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &wgpu::RenderPipeline,
    vertices: &[MiditrailVertex],
) {
    if vertices.is_empty() {
        return;
    }
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("MiditrailColorVertices"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    pass.set_pipeline(pipeline);
    pass.set_vertex_buffer(0, buffer.slice(..));
    pass.draw(0..vertices.len() as u32, 0..1);
}

fn draw_aura(
    device: &wgpu::Device,
    pass: &mut wgpu::RenderPass<'_>,
    pipeline: &wgpu::RenderPipeline,
    vertices: &[MiditrailAuraVertex],
) {
    if vertices.is_empty() {
        return;
    }
    let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("MiditrailAuraVertices"),
        contents: bytemuck::cast_slice(vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    pass.set_pipeline(pipeline);
    pass.set_vertex_buffer(0, buffer.slice(..));
    pass.draw(0..vertices.len() as u32, 0..1);
}

fn build_mvp(config: &MiditrailSceneConfig, aspect: f32) -> Mat4 {
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
            -config.view_offset,
        ))
        * model
}

fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("MiditrailDepthTexture"),
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
        label: Some("MiditrailHeadlessTarget"),
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
