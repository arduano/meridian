use bytemuck::cast_slice;

use crate::render::{SceneQuad, shared::NoteInstance};

use super::renderer::PrimitiveSceneRenderer;

pub(super) fn submit_note_chunks(
    renderer: &mut PrimitiveSceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    depth_view: Option<&wgpu::TextureView>,
    pipeline: &wgpu::RenderPipeline,
    notes: &[NoteInstance],
    has_existing_color: bool,
    clear_color: wgpu::Color,
    label: &'static str,
) -> bool {
    let mut rendered_any = false;
    for (chunk_index, chunk) in notes.chunks(renderer.note_instance_capacity).enumerate() {
        queue.write_buffer(
            &renderer.resources.note_instance_buffer,
            0,
            cast_slice(chunk),
        );
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        let color_load = if has_existing_color || rendered_any || chunk_index > 0 {
            wgpu::LoadOp::Load
        } else {
            wgpu::LoadOp::Clear(clear_color)
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
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &renderer.resources.note_bind_group, &[]);
        pass.set_vertex_buffer(0, renderer.resources.note_vertex_buffer.slice(..));
        pass.set_vertex_buffer(
            1,
            renderer
                .resources
                .note_instance_buffer
                .slice(..(chunk.len() * std::mem::size_of::<NoteInstance>()) as u64),
        );
        pass.draw(0..6, 0..chunk.len() as u32);
        drop(pass);
        queue.submit(Some(encoder.finish()));
        rendered_any = true;
    }
    rendered_any
}

pub(super) fn submit_quad_chunks(
    renderer: &mut PrimitiveSceneRenderer,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    depth_view: Option<&wgpu::TextureView>,
    quads: &[SceneQuad],
    has_existing_color: bool,
    clear_color: wgpu::Color,
    label: &'static str,
) -> bool {
    let mut rendered_any = false;
    for (chunk_index, chunk) in quads.chunks(renderer.quad_instance_capacity).enumerate() {
        queue.write_buffer(
            &renderer.resources.quad_instance_buffer,
            0,
            cast_slice(chunk),
        );
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) });
        let color_load = if has_existing_color || rendered_any || chunk_index > 0 {
            wgpu::LoadOp::Load
        } else {
            wgpu::LoadOp::Clear(clear_color)
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
        pass.set_pipeline(&renderer.resources.flat_pipeline);
        pass.set_vertex_buffer(0, renderer.resources.quad_vertex_buffer.slice(..));
        pass.set_vertex_buffer(
            1,
            renderer
                .resources
                .quad_instance_buffer
                .slice(..(chunk.len() * std::mem::size_of::<SceneQuad>()) as u64),
        );
        pass.draw(0..6, 0..chunk.len() as u32);
        drop(pass);
        queue.submit(Some(encoder.finish()));
        rendered_any = true;
    }
    rendered_any
}

pub(super) fn clear_target(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    clear_color: wgpu::Color,
) {
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
                    load: wgpu::LoadOp::Clear(clear_color),
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
