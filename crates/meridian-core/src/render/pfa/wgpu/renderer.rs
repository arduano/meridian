use bytemuck::cast_slice;

use crate::render::{
    SceneLayer, SceneQuad,
    shared::{NoteInstance, NoteShaderKind, ProjectedScene},
};

use super::{passes, pipeline::*};

pub struct PrimitiveSceneRenderer {
    pub(super) resources: RendererResources,
    pub(super) quad_instance_capacity: usize,
    pub(super) note_instance_capacity: usize,
}

impl PrimitiveSceneRenderer {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            resources: create_renderer_resources(device),
            quad_instance_capacity: MIN_STREAMING_INSTANCE_QUADS,
            note_instance_capacity: MIN_STREAMING_INSTANCE_QUADS,
        }
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
            .resources
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
        self.write_note_uniforms(queue, scene);

        let note_layers = if scene.notes_black_first {
            [SceneLayer::BlackNotes, SceneLayer::WhiteNotes]
        } else {
            [SceneLayer::WhiteNotes, SceneLayer::BlackNotes]
        };
        let note_pipeline = match scene.note_shader_kind() {
            NoteShaderKind::Flat => self.resources.flat_note_pipeline.clone(),
            NoteShaderKind::Pfa => self.resources.pfa_note_pipeline.clone(),
        };
        let mut has_color = false;
        for layer in note_layers {
            has_color = passes::submit_note_chunks(
                self,
                device,
                queue,
                &view,
                Some(&depth_view),
                &note_pipeline,
                scene.note_layer(layer),
                has_color,
                "MeridianNoteChunk",
            ) || has_color;
        }

        let flat_layers = if scene.notes_black_first {
            [
                SceneLayer::Background,
                SceneLayer::BlackNotes,
                SceneLayer::WhiteNotes,
                SceneLayer::KeyboardDecor,
                SceneLayer::WhiteKeys,
                SceneLayer::BlackKeys,
                SceneLayer::Overlay,
            ]
        } else {
            [
                SceneLayer::Background,
                SceneLayer::WhiteNotes,
                SceneLayer::BlackNotes,
                SceneLayer::KeyboardDecor,
                SceneLayer::WhiteKeys,
                SceneLayer::BlackKeys,
                SceneLayer::Overlay,
            ]
        };
        for layer in flat_layers {
            has_color = passes::submit_quad_chunks(
                self,
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
            passes::clear_target(device, queue, &view);
        }
    }

    fn write_note_uniforms(&self, queue: &wgpu::Queue, scene: &ProjectedScene) {
        let note_params = scene.note_params();
        queue.write_buffer(
            &self.resources.note_params_buffer,
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
        queue.write_buffer(
            &self.resources.note_key_buffer,
            0,
            cast_slice(&key_positions),
        );
    }

    fn ensure_quad_instance_capacity(&mut self, device: &wgpu::Device, required: usize) {
        if required <= self.quad_instance_capacity {
            return;
        }
        let new_capacity = required.next_power_of_two().max(1024);
        self.resources.quad_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
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
        self.resources.note_instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("MeridianNoteInstances"),
            size: (new_capacity * std::mem::size_of::<NoteInstance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        self.note_instance_capacity = new_capacity;
    }

    fn ensure_depth_texture(&mut self, device: &wgpu::Device, target: &wgpu::Texture) {
        let size = target.size();
        if size == self.resources.depth_extent {
            return;
        }
        self.resources.depth_texture = create_depth_texture(device, size);
        self.resources.depth_extent = size;
    }
}
