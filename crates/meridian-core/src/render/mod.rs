mod pfa;
pub mod wgpu;

use bytemuck::{Pod, Zeroable};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::midi::{MIDI_KEY_COUNT, MIDIFileUnion};
use pfa::PfaProjector;

const LAYER_COUNT: usize = 7;
const NOTE_DEPTH_STEP: f32 = 1.0 / 16_777_216.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererKind {
    Basic,
    Pfa,
}

#[repr(usize)]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SceneLayer {
    Background = 0,
    WhiteNotes = 1,
    BlackNotes = 2,
    KeyboardDecor = 3,
    WhiteKeys = 4,
    BlackKeys = 5,
    Overlay = 6,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]
pub struct SceneQuad {
    pub positions: [[f32; 2]; 4],
    pub colors: [[f32; 4]; 4],
    pub depth: f32,
    pub _padding: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]
pub struct NoteInstance {
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub left_color: u32,
    pub right_color: u32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub depth: f32,
    pub _padding: [u32; 3],
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SceneLayout {
    pub renderer: RendererKind,
    pub view_range: f64,
    pub piano_height: f32,
    pub first_key: u8,
    pub last_key: u8,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl Default for SceneLayout {
    fn default() -> Self {
        Self {
            renderer: RendererKind::Pfa,
            view_range: 8.0,
            piano_height: 0.151,
            first_key: 0,
            last_key: 127,
            viewport_width: 1280,
            viewport_height: 720,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjectedScene {
    layers: [Vec<SceneQuad>; LAYER_COUNT],
    note_layers: [Vec<NoteInstance>; 2],
    pub notes_black_first: bool,
    pub visible_notes: usize,
    pub active_keys: usize,
    pub note_quads: usize,
    pub keyboard_quads: usize,
}

impl Default for ProjectedScene {
    fn default() -> Self {
        Self {
            layers: std::array::from_fn(|_| Vec::new()),
            note_layers: std::array::from_fn(|_| Vec::new()),
            notes_black_first: false,
            visible_notes: 0,
            active_keys: 0,
            note_quads: 0,
            keyboard_quads: 0,
        }
    }
}

impl ProjectedScene {
    pub fn clear(&mut self) {
        for layer in &mut self.layers {
            layer.clear();
        }
        for layer in &mut self.note_layers {
            layer.clear();
        }
        self.notes_black_first = false;
        self.visible_notes = 0;
        self.active_keys = 0;
        self.note_quads = 0;
        self.keyboard_quads = 0;
    }

    pub fn push_quad(&mut self, layer: SceneLayer, quad: SceneQuad) {
        self.layers[layer as usize].push(quad);
    }

    pub fn push_pfa_note(
        &mut self,
        layer: SceneLayer,
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        left_color: [f32; 4],
        right_color: [f32; 4],
        pad_x: f32,
        pad_y: f32,
        depth: f32,
    ) {
        let note_layer = match layer {
            SceneLayer::WhiteNotes => &mut self.note_layers[0],
            SceneLayer::BlackNotes => &mut self.note_layers[1],
            _ => {
                self.push_quad(
                    layer,
                    vertical_gradient_quad_with_depth(
                        x1,
                        y1,
                        x2,
                        y2,
                        left_color,
                        right_color,
                        right_color,
                        left_color,
                        depth,
                    ),
                );
                return;
            }
        };
        note_layer.push(NoteInstance {
            x1,
            y1,
            x2,
            y2,
            left_color: pack_rgba8(left_color),
            right_color: pack_rgba8(right_color),
            pad_x,
            pad_y,
            depth,
            _padding: [0; 3],
        });
    }

    pub fn layer(&self, layer: SceneLayer) -> &[SceneQuad] {
        &self.layers[layer as usize]
    }

    pub fn note_layer(&self, layer: SceneLayer) -> &[NoteInstance] {
        match layer {
            SceneLayer::WhiteNotes => &self.note_layers[0],
            SceneLayer::BlackNotes => &self.note_layers[1],
            _ => &[],
        }
    }

    pub(crate) fn extend_note_layer(&mut self, layer: SceneLayer, notes: Vec<NoteInstance>) {
        match layer {
            SceneLayer::WhiteNotes => self.note_layers[0].extend(notes),
            SceneLayer::BlackNotes => self.note_layers[1].extend(notes),
            _ => {}
        }
    }

    pub fn total_quads(&self) -> usize {
        self.layers.iter().map(Vec::len).sum::<usize>()
            + self.note_layers.iter().map(Vec::len).sum::<usize>()
    }

    pub fn total_vertices(&self) -> usize {
        self.total_quads() * 6
    }
}

pub fn project_scene(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    layout: &SceneLayout,
) -> ProjectedScene {
    let mut scene = ProjectedScene::default();
    project_scene_into(midi, current_time, layout, &mut scene);
    scene
}

pub fn project_scene_into(
    midi: &mut MIDIFileUnion,
    current_time: f64,
    layout: &SceneLayout,
    scene: &mut ProjectedScene,
) {
    scene.clear();
    match layout.renderer {
        RendererKind::Basic => BasicProjector.project_into(midi, current_time, layout, scene),
        RendererKind::Pfa => {
            PfaProjector::default().project_into(midi, current_time, layout, scene)
        }
    }
}

pub(crate) trait SceneProjector {
    fn project_into(
        &self,
        midi: &mut MIDIFileUnion,
        current_time: f64,
        layout: &SceneLayout,
        scene: &mut ProjectedScene,
    );
}

struct BasicProjector;

impl SceneProjector for BasicProjector {
    fn project_into(
        &self,
        midi: &mut MIDIFileUnion,
        current_time: f64,
        layout: &SceneLayout,
        scene: &mut ProjectedScene,
    ) {
        scene.notes_black_first = false;
        let views = midi.get_current_column_views(current_time, layout.view_range);
        let first_key = layout.first_key.min(layout.last_key) as usize;
        let last_key = layout.last_key.max(layout.first_key) as usize;
        let view_range = views.range().length() as f32;
        let piano_height = layout.piano_height;
        let key_width = 1.0 / ((last_key - first_key + 1) as f32);

        for key in first_key..=last_key.min(MIDI_KEY_COUNT - 1) {
            let x1 = (key - first_key) as f32 * key_width;
            let x2 = x1 + key_width;
            let is_black = is_black_key(key as u8);
            let mut key_color = None;
            let column = views.get_column(key);

            column.for_each_displaced_note(|note| {
                let end = note.start + note.len;
                if end <= 0.0 || note.start >= view_range {
                    return;
                }

                let bottom =
                    piano_height + (note.start.max(0.0) / view_range) * (1.0 - piano_height);
                let top = piano_height + (end.min(view_range) / view_range) * (1.0 - piano_height);
                let color = note.color.to_rgba(if is_black { 0.94 } else { 0.88 });
                let border = shade(color, if is_black { 0.18 } else { -0.18 });
                let inset = key_width * if is_black { 0.10 } else { 0.06 };

                scene.push_quad(
                    if is_black {
                        SceneLayer::BlackNotes
                    } else {
                        SceneLayer::WhiteNotes
                    },
                    vertical_gradient_quad(x1, bottom, x2, top, border, border, border, border),
                );
                scene.push_quad(
                    if is_black {
                        SceneLayer::BlackNotes
                    } else {
                        SceneLayer::WhiteNotes
                    },
                    vertical_gradient_quad(
                        x1 + inset,
                        bottom + 0.003,
                        x2 - inset,
                        (top - 0.003).max(bottom + 0.003),
                        color,
                        color,
                        shade(color, 0.08),
                        shade(color, 0.08),
                    ),
                );

                if note.start <= 0.0 && end > 0.0 {
                    key_color = Some(color);
                }
                scene.visible_notes += 1;
                scene.note_quads += 2;
            });

            let key_top = if is_black {
                piano_height * 0.62
            } else {
                piano_height
            };
            let base = if is_black {
                [0.08, 0.16, 0.09, 1.0]
            } else {
                [0.74, 0.96, 0.76, 1.0]
            };
            let fill = key_color
                .map(|color| mix(base, color, if is_black { 0.75 } else { 0.45 }))
                .unwrap_or(base);
            let border = if is_black {
                shade(fill, 0.12)
            } else {
                shade(fill, -0.18)
            };
            if key_color.is_some() {
                scene.active_keys += 1;
            }

            scene.push_quad(
                if is_black {
                    SceneLayer::BlackKeys
                } else {
                    SceneLayer::WhiteKeys
                },
                vertical_gradient_quad(x1, 0.0, x2, key_top, border, border, border, border),
            );
            scene.push_quad(
                if is_black {
                    SceneLayer::BlackKeys
                } else {
                    SceneLayer::WhiteKeys
                },
                vertical_gradient_quad(
                    x1 + key_width * if is_black { 0.08 } else { 0.03 },
                    piano_height * 0.012,
                    x2 - key_width * if is_black { 0.08 } else { 0.03 },
                    key_top - piano_height * 0.012,
                    fill,
                    fill,
                    fill,
                    fill,
                ),
            );
            scene.keyboard_quads += 2;
        }
    }
}

fn vertical_gradient_quad(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    c_bl: [f32; 4],
    c_br: [f32; 4],
    c_tr: [f32; 4],
    c_tl: [f32; 4],
) -> SceneQuad {
    quad(
        [[x1, y1], [x2, y1], [x2, y2], [x1, y2]],
        [c_bl, c_br, c_tr, c_tl],
    )
}

fn quad(positions: [[f32; 2]; 4], colors: [[f32; 4]; 4]) -> SceneQuad {
    SceneQuad {
        positions,
        colors,
        depth: 0.5,
        _padding: [0.0; 3],
    }
}

fn quad_with_depth(positions: [[f32; 2]; 4], colors: [[f32; 4]; 4], depth: f32) -> SceneQuad {
    SceneQuad {
        positions,
        colors,
        depth,
        _padding: [0.0; 3],
    }
}

fn vertical_gradient_quad_with_depth(
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    c_bl: [f32; 4],
    c_br: [f32; 4],
    c_tr: [f32; 4],
    c_tl: [f32; 4],
    depth: f32,
) -> SceneQuad {
    quad_with_depth(
        [[x1, y1], [x2, y1], [x2, y2], [x1, y2]],
        [c_bl, c_br, c_tr, c_tl],
        depth,
    )
}

fn mod_add(color: [f32; 4], add: f32, mul: f32) -> [f32; 4] {
    [
        (color[0] * mul + add).clamp(0.0, 1.0),
        (color[1] * mul + add).clamp(0.0, 1.0),
        (color[2] * mul + add).clamp(0.0, 1.0),
        color[3],
    ]
}

fn shade(color: [f32; 4], amount: f32) -> [f32; 4] {
    [
        (color[0] + amount).clamp(0.0, 1.0),
        (color[1] + amount).clamp(0.0, 1.0),
        (color[2] + amount).clamp(0.0, 1.0),
        color[3],
    ]
}

fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

fn is_black_key(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}

fn pack_rgba8(color: [f32; 4]) -> u32 {
    let to_byte = |component: f32| (component.clamp(0.0, 1.0) * 255.0).round() as u32;
    to_byte(color[0])
        | (to_byte(color[1]) << 8)
        | (to_byte(color[2]) << 16)
        | (to_byte(color[3]) << 24)
}
