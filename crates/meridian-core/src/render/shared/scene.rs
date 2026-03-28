use crate::midi::MIDI_KEY_COUNT;

use super::{LAYER_COUNT, SceneLayer, SceneQuad};
use crate::render::pfa::NoteInstance;

#[derive(Clone, Copy, Debug, Default)]
pub struct KeyActivity {
    pub pressed: bool,
    pub left: [f32; 4],
    pub right: [f32; 4],
}

#[derive(Clone, Debug)]
pub struct ProjectedScene {
    layers: [Vec<SceneQuad>; LAYER_COUNT],
    note_layers: [Vec<NoteInstance>; 2],
    note_key_x: [[f32; 2]; MIDI_KEY_COUNT],
    note_params: [f32; 4],
    key_activity: [KeyActivity; MIDI_KEY_COUNT],
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
            key_activity: [KeyActivity::default(); MIDI_KEY_COUNT],
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
        self.key_activity.fill(KeyActivity::default());
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

    pub fn key_activity(&self, key: usize) -> KeyActivity {
        self.key_activity[key]
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

    pub(crate) fn set_key_activity(&mut self, key: usize, activity: KeyActivity) {
        self.key_activity[key] = activity;
    }

    pub fn total_quads(&self) -> usize {
        self.layers.iter().map(Vec::len).sum::<usize>()
            + self.note_layers.iter().map(Vec::len).sum::<usize>()
    }

    pub fn total_vertices(&self) -> usize {
        self.total_quads() * 6
    }
}
