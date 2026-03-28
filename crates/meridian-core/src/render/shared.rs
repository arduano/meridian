use serde::{Deserialize, Serialize};

use crate::midi::MIDI_KEY_COUNT;

use super::pfa::NoteInstance;

pub(crate) const LAYER_COUNT: usize = 7;
pub const DEFAULT_PFA_KEYBOARD_ASPECT_RATIO: f32 = 0.084_937_5;

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererKind {
    Basic,
    Flat,
    Pfa,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PfaTopColor {
    Red,
    Blue,
    Green,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasicNoteProjectorConfig;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatNoteProjectorConfig;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PfaNoteProjectorConfig {
    #[serde(default)]
    pub same_width_notes: bool,
    #[serde(default = "default_true")]
    pub black_notes_above: bool,
    #[serde(default = "default_border_width")]
    pub border_width: f32,
}

impl Default for PfaNoteProjectorConfig {
    fn default() -> Self {
        Self {
            same_width_notes: false,
            black_notes_above: true,
            border_width: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct BasicKeyboardProjectorConfig;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatKeyboardProjectorConfig;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PfaKeyboardProjectorConfig {
    #[serde(default)]
    pub same_width_notes: bool,
    #[serde(default)]
    pub middle_c: bool,
    #[serde(default)]
    pub top_color: PfaTopColor,
    #[serde(default = "default_top_bar_rgb")]
    pub top_bar_rgb: [f32; 3],
}

impl Default for PfaKeyboardProjectorConfig {
    fn default() -> Self {
        Self {
            same_width_notes: false,
            middle_c: false,
            top_color: PfaTopColor::Red,
            top_bar_rgb: default_top_bar_rgb(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "projector", rename_all = "snake_case")]
pub enum NoteProjectorConfig {
    Basic(BasicNoteProjectorConfig),
    Flat(FlatNoteProjectorConfig),
    Pfa(PfaNoteProjectorConfig),
}

impl Default for NoteProjectorConfig {
    fn default() -> Self {
        Self::Pfa(PfaNoteProjectorConfig::default())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "projector", rename_all = "snake_case")]
pub enum KeyboardProjectorConfig {
    Basic(BasicKeyboardProjectorConfig),
    Flat(FlatKeyboardProjectorConfig),
    Pfa(PfaKeyboardProjectorConfig),
}

impl Default for KeyboardProjectorConfig {
    fn default() -> Self {
        Self::Pfa(PfaKeyboardProjectorConfig::default())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum KeyboardHeightSpec {
    ScreenPercent { height: f32 },
    AspectRatio { ratio: f32 },
}

impl KeyboardHeightSpec {
    pub fn resolve(&self, viewport_width: u32, viewport_height: u32) -> f32 {
        match *self {
            Self::ScreenPercent { height } => height.clamp(0.02, 0.95),
            Self::AspectRatio { ratio } => {
                let ratio = ratio.max(0.001);
                (ratio * viewport_width as f32 / viewport_height.max(1) as f32).clamp(0.02, 0.95)
            }
        }
    }
}

impl Default for KeyboardHeightSpec {
    fn default() -> Self {
        Self::AspectRatio {
            ratio: DEFAULT_PFA_KEYBOARD_ASPECT_RATIO,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TwoDSceneConfig {
    #[serde(default)]
    pub keyboard_height: KeyboardHeightSpec,
    #[serde(default)]
    pub notes: NoteProjectorConfig,
    #[serde(default)]
    pub keyboard: KeyboardProjectorConfig,
}

impl Default for TwoDSceneConfig {
    fn default() -> Self {
        Self {
            keyboard_height: KeyboardHeightSpec::default(),
            notes: NoteProjectorConfig::default(),
            keyboard: KeyboardProjectorConfig::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ThreeDSceneConfig {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "scene_type", rename_all = "snake_case")]
pub enum SceneConfig {
    TwoD(TwoDSceneConfig),
    ThreeD(ThreeDSceneConfig),
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self::TwoD(TwoDSceneConfig::default())
    }
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneLayout {
    pub scene: SceneConfig,
    pub view_range: f64,
    pub first_key: u8,
    pub last_key: u8,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

impl Default for SceneLayout {
    fn default() -> Self {
        Self {
            scene: SceneConfig::default(),
            view_range: 8.0,
            first_key: 0,
            last_key: 127,
            viewport_width: 1280,
            viewport_height: 720,
        }
    }
}

impl Default for PfaTopColor {
    fn default() -> Self {
        Self::Red
    }
}

impl SceneLayout {
    pub fn piano_height(&self) -> f32 {
        match &self.scene {
            SceneConfig::TwoD(scene) => {
                scene.keyboard_height.resolve(self.viewport_width, self.viewport_height)
            }
            SceneConfig::ThreeD(_) => 0.151,
        }
    }

    pub fn set_renderer_kind(&mut self, renderer: RendererKind) {
        self.scene = SceneConfig::TwoD(match renderer {
            RendererKind::Basic => TwoDSceneConfig {
                keyboard_height: KeyboardHeightSpec::default(),
                notes: NoteProjectorConfig::Basic(BasicNoteProjectorConfig),
                keyboard: KeyboardProjectorConfig::Basic(BasicKeyboardProjectorConfig),
            },
            RendererKind::Flat => TwoDSceneConfig {
                keyboard_height: KeyboardHeightSpec::default(),
                notes: NoteProjectorConfig::Flat(FlatNoteProjectorConfig),
                keyboard: KeyboardProjectorConfig::Flat(FlatKeyboardProjectorConfig),
            },
            RendererKind::Pfa => TwoDSceneConfig::default(),
        });
    }

    pub fn legacy_renderer_kind(&self) -> Option<RendererKind> {
        let SceneConfig::TwoD(scene) = &self.scene else {
            return None;
        };
        match (&scene.notes, &scene.keyboard) {
            (NoteProjectorConfig::Basic(_), KeyboardProjectorConfig::Basic(_)) => {
                Some(RendererKind::Basic)
            }
            (NoteProjectorConfig::Flat(_), KeyboardProjectorConfig::Flat(_)) => {
                Some(RendererKind::Flat)
            }
            (NoteProjectorConfig::Pfa(_), KeyboardProjectorConfig::Pfa(_)) => {
                Some(RendererKind::Pfa)
            }
            _ => None,
        }
    }
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

pub(crate) fn solid_quad(x1: f32, y1: f32, x2: f32, y2: f32, color: [f32; 4]) -> SceneQuad {
    quad(
        [[x1, y1], [x2, y1], [x2, y2], [x1, y2]],
        [color, color, color, color],
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

pub(crate) fn alpha_blend(top: [f32; 4], bottom: [f32; 4]) -> [f32; 4] {
    let alpha = top[3].clamp(0.0, 1.0);
    [
        top[0] * alpha + bottom[0] * (1.0 - alpha),
        top[1] * alpha + bottom[1] * (1.0 - alpha),
        top[2] * alpha + bottom[2] * (1.0 - alpha),
        1.0,
    ]
}

const fn default_true() -> bool {
    true
}

const fn default_border_width() -> f32 {
    1.0
}

const fn default_top_bar_rgb() -> [f32; 3] {
    [0.585, 0.0392, 0.0249]
}
