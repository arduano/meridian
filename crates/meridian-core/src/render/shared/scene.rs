use crate::midi::MIDI_KEY_COUNT;

use super::{LAYER_COUNT, SceneLayer, SceneQuad};
use crate::render::piano_trail_classic::PianoTrailClassicScene;

use super::NoteInstance;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum NoteShaderKind {
    #[default]
    Flat,
    Pfa,
}

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
    piano_trail_classic: Option<PianoTrailClassicScene>,
    note_key_x: [[f32; 2]; MIDI_KEY_COUNT],
    note_params: [f32; 4],
    note_shader_kind: NoteShaderKind,
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
            piano_trail_classic: None,
            note_key_x: [[0.0; 2]; MIDI_KEY_COUNT],
            note_params: [0.0; 4],
            note_shader_kind: NoteShaderKind::default(),
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
        if let Some(piano_trail_classic) = &mut self.piano_trail_classic {
            piano_trail_classic.clear();
        }
        self.key_activity.fill(KeyActivity::default());
        self.note_shader_kind = NoteShaderKind::default();
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

    pub fn piano_trail_classic(&self) -> Option<&PianoTrailClassicScene> {
        self.piano_trail_classic.as_ref()
    }

    pub(crate) fn take_piano_trail_classic(&mut self) -> Option<PianoTrailClassicScene> {
        self.piano_trail_classic.take()
    }

    pub fn note_key_x(&self) -> &[[f32; 2]; MIDI_KEY_COUNT] {
        &self.note_key_x
    }

    pub fn note_params(&self) -> [f32; 4] {
        self.note_params
    }

    pub(crate) fn note_shader_kind(&self) -> NoteShaderKind {
        self.note_shader_kind
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

    pub(crate) fn set_note_shader_kind(&mut self, kind: NoteShaderKind) {
        self.note_shader_kind = kind;
    }

    pub(crate) fn extend_note_layer(&mut self, layer: SceneLayer, notes: Vec<NoteInstance>) {
        match layer {
            SceneLayer::WhiteNotes => self.note_layers[0].extend(notes),
            SceneLayer::BlackNotes => self.note_layers[1].extend(notes),
            _ => {}
        }
    }

    pub(crate) fn push_note_layer(&mut self, layer: SceneLayer, note: NoteInstance) {
        match layer {
            SceneLayer::WhiteNotes => self.note_layers[0].push(note),
            SceneLayer::BlackNotes => self.note_layers[1].push(note),
            _ => {}
        }
    }

    pub(crate) fn set_key_activity(&mut self, key: usize, activity: KeyActivity) {
        self.key_activity[key] = activity;
    }

    pub(crate) fn set_piano_trail_classic(&mut self, piano_trail_classic: PianoTrailClassicScene) {
        self.piano_trail_classic = Some(piano_trail_classic);
    }

    pub fn total_quads(&self) -> usize {
        let base = self.layers.iter().map(Vec::len).sum::<usize>()
            + self.note_layers.iter().map(Vec::len).sum::<usize>();
        if self.piano_trail_classic.is_some() {
            self.note_quads + self.keyboard_quads
        } else {
            base
        }
    }

    pub fn total_vertices(&self) -> usize {
        if let Some(piano_trail_classic) = &self.piano_trail_classic {
            piano_trail_classic.total_vertices()
        } else {
            self.total_quads() * 6
        }
    }
}
