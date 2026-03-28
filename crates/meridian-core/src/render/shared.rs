use serde::{Deserialize, Serialize};

use crate::midi::MIDI_KEY_COUNT;

use super::pfa::NoteInstance;

pub(crate) const LAYER_COUNT: usize = 7;
pub(crate) const NOTE_DEPTH_STEP: f32 = 1.0 / 16_777_216.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable, Serialize, Deserialize)]
pub struct SceneQuad {
    pub positions: [[f32; 2]; 4],
    pub colors: [[f32; 4]; 4],
    pub depth: f32,
    pub _padding: [f32; 3],
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

#[derive(Clone, Debug)]
pub struct ProjectedScene {
    layers: [Vec<SceneQuad>; LAYER_COUNT],
    note_layers: [Vec<NoteInstance>; 2],
    note_key_x: [[f32; 2]; MIDI_KEY_COUNT],
    note_params: [f32; 4],
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
            note_key_x: [[0.0; 2]; MIDI_KEY_COUNT],
            note_params: [0.0; 4],
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

    pub fn note_key_x(&self) -> &[[f32; 2]; MIDI_KEY_COUNT] {
        &self.note_key_x
    }

    pub fn note_params(&self) -> [f32; 4] {
        self.note_params
    }

    pub(crate) fn set_note_key_x(&mut self, key: u8, x1: f32, x2: f32) {
        self.note_key_x[key as usize] = [x1, x2];
    }

    pub(crate) fn set_note_params(
        &mut self,
        piano_height: f32,
        note_pos_factor: f32,
        pad_x: f32,
        pad_y: f32,
    ) {
        self.note_params = [piano_height, note_pos_factor, pad_x, pad_y];
    }

    pub(crate) fn extend_note_layer(&mut self, layer: SceneLayer, notes: Vec<NoteInstance>) {
        match layer {
            SceneLayer::WhiteNotes => self.note_layers[0].extend(notes),
            SceneLayer::BlackNotes => self.note_layers[1].extend(notes),
            _ => {}
        }
    }

    pub fn total_quads(&self) -> usize {
        self.layers.iter().map(Vec::len).sum::<usize>() + self.note_quads
    }

    pub fn total_vertices(&self) -> usize {
        self.total_quads() * 6
    }
}

pub(crate) fn vertical_gradient_quad(
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

pub(crate) fn quad(positions: [[f32; 2]; 4], colors: [[f32; 4]; 4]) -> SceneQuad {
    SceneQuad {
        positions,
        colors,
        depth: 0.5,
        _padding: [0.0; 3],
    }
}

pub(crate) fn quad_with_depth(
    positions: [[f32; 2]; 4],
    colors: [[f32; 4]; 4],
    depth: f32,
) -> SceneQuad {
    SceneQuad {
        positions,
        colors,
        depth,
        _padding: [0.0; 3],
    }
}

pub(crate) fn vertical_gradient_quad_with_depth(
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

pub(crate) fn mod_add(color: [f32; 4], add: f32, mul: f32) -> [f32; 4] {
    [
        (color[0] * mul + add).clamp(0.0, 1.0),
        (color[1] * mul + add).clamp(0.0, 1.0),
        (color[2] * mul + add).clamp(0.0, 1.0),
        color[3],
    ]
}

pub(crate) fn shade(color: [f32; 4], amount: f32) -> [f32; 4] {
    [
        (color[0] + amount).clamp(0.0, 1.0),
        (color[1] + amount).clamp(0.0, 1.0),
        (color[2] + amount).clamp(0.0, 1.0),
        color[3],
    ]
}

pub(crate) fn mix(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}

pub(crate) fn is_black_key(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}
