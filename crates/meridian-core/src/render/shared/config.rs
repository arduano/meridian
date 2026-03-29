use serde::{Deserialize, Serialize};

use super::{NotePaletteConfig, ProjectorImageConfig};

pub const DEFAULT_PFA_KEYBOARD_ASPECT_RATIO: f32 = 0.084_937_5;

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RendererKind {
    Flat,
    Pfa,
    Miditrail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PfaTopColor {
    Red,
    Blue,
    Green,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlatNoteProjectorConfig {
    #[serde(default)]
    pub palette: NotePaletteConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PfaNoteProjectorConfig {
    #[serde(default)]
    pub same_width_notes: bool,
    #[serde(default = "default_border_width")]
    pub border_width: f32,
    #[serde(default)]
    pub palette: NotePaletteConfig,
}

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "projector", rename_all = "snake_case")]
pub enum NoteProjectorConfig {
    Flat(FlatNoteProjectorConfig),
    Pfa(PfaNoteProjectorConfig),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "projector", rename_all = "snake_case")]
pub enum KeyboardProjectorConfig {
    Flat(FlatKeyboardProjectorConfig),
    Pfa(PfaKeyboardProjectorConfig),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum KeyboardHeightSpec {
    ScreenPercent { height: f32 },
    AspectRatio { ratio: f32 },
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiditrailSceneConfig {
    #[serde(default = "default_miditrail_same_width_notes")]
    pub same_width_notes: bool,
    #[serde(default = "default_miditrail_fov")]
    pub fov: f32,
    #[serde(default = "default_miditrail_view_height")]
    pub view_height: f32,
    #[serde(default = "default_miditrail_view_offset")]
    pub view_offset: f32,
    #[serde(default)]
    pub view_pan: f32,
    #[serde(default = "default_miditrail_cam_ang")]
    pub cam_ang: f32,
    #[serde(default)]
    pub cam_rot: f32,
    #[serde(default)]
    pub cam_spin: f32,
    #[serde(default = "default_miditrail_viewdist")]
    pub viewdist: f32,
    #[serde(default = "default_miditrail_viewback")]
    pub viewback: f32,
    #[serde(default)]
    pub vertical_notes: bool,
    #[serde(default = "default_miditrail_note_down_speed")]
    pub note_down_speed: f32,
    #[serde(default = "default_miditrail_note_up_speed")]
    pub note_up_speed: f32,
    #[serde(default)]
    pub box_notes: bool,
    #[serde(default)]
    pub light_shade: bool,
    #[serde(default = "default_miditrail_show_keyboard")]
    pub show_keyboard: bool,
    #[serde(default = "default_miditrail_tilt_keys")]
    pub tilt_keys: bool,
    #[serde(default)]
    pub eat_notes: bool,
    #[serde(default = "default_miditrail_aura_strength")]
    pub aura_strength: f32,
    #[serde(default = "default_miditrail_aura_enabled")]
    pub aura_enabled: bool,
    #[serde(default)]
    pub notes_change_size: bool,
    #[serde(default = "default_miditrail_notes_change_tint")]
    pub notes_change_tint: bool,
    #[serde(default)]
    pub use_vel: bool,
    #[serde(default)]
    pub palette: NotePaletteConfig,
    #[serde(default = "default_miditrail_aura_image")]
    pub aura_image: ProjectorImageConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "projector", rename_all = "snake_case")]
pub enum ThreeDSceneConfig {
    Miditrail(MiditrailSceneConfig),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "scene_type", rename_all = "snake_case")]
pub enum SceneConfig {
    TwoD(TwoDSceneConfig),
    ThreeD(ThreeDSceneConfig),
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

impl Default for PfaNoteProjectorConfig {
    fn default() -> Self {
        Self {
            same_width_notes: false,
            border_width: 1.0,
            palette: NotePaletteConfig::default(),
        }
    }
}

impl Default for FlatNoteProjectorConfig {
    fn default() -> Self {
        Self {
            palette: NotePaletteConfig::default(),
        }
    }
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

impl Default for NoteProjectorConfig {
    fn default() -> Self {
        Self::Pfa(PfaNoteProjectorConfig::default())
    }
}

impl Default for KeyboardProjectorConfig {
    fn default() -> Self {
        Self::Pfa(PfaKeyboardProjectorConfig::default())
    }
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

impl Default for TwoDSceneConfig {
    fn default() -> Self {
        Self {
            keyboard_height: KeyboardHeightSpec::default(),
            notes: NoteProjectorConfig::default(),
            keyboard: KeyboardProjectorConfig::default(),
        }
    }
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self::TwoD(TwoDSceneConfig::default())
    }
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

impl Default for MiditrailSceneConfig {
    fn default() -> Self {
        Self {
            same_width_notes: default_miditrail_same_width_notes(),
            fov: default_miditrail_fov(),
            view_height: default_miditrail_view_height(),
            view_offset: default_miditrail_view_offset(),
            view_pan: 0.0,
            cam_ang: default_miditrail_cam_ang(),
            cam_rot: 0.0,
            cam_spin: 0.0,
            viewdist: default_miditrail_viewdist(),
            viewback: default_miditrail_viewback(),
            vertical_notes: false,
            note_down_speed: default_miditrail_note_down_speed(),
            note_up_speed: default_miditrail_note_up_speed(),
            box_notes: false,
            light_shade: false,
            show_keyboard: default_miditrail_show_keyboard(),
            tilt_keys: default_miditrail_tilt_keys(),
            eat_notes: false,
            aura_strength: default_miditrail_aura_strength(),
            aura_enabled: default_miditrail_aura_enabled(),
            notes_change_size: false,
            notes_change_tint: default_miditrail_notes_change_tint(),
            use_vel: false,
            palette: NotePaletteConfig::default(),
            aura_image: default_miditrail_aura_image(),
        }
    }
}

impl Default for ThreeDSceneConfig {
    fn default() -> Self {
        Self::Miditrail(MiditrailSceneConfig::default())
    }
}

impl PfaTopColor {
    pub fn preset_bar_rgb(self) -> Option<[f32; 3]> {
        match self {
            Self::Red => None,
            Self::Blue => Some([0.0392, 0.0249, 0.585]),
            Self::Green => Some([0.0249, 0.585, 0.0392]),
        }
    }
}

impl PfaKeyboardProjectorConfig {
    pub fn resolved_top_bar_rgb(&self) -> [f32; 3] {
        self.top_color.preset_bar_rgb().unwrap_or(self.top_bar_rgb)
    }

    pub fn resolved_top_bar_gradient(&self) -> ([f32; 4], [f32; 4]) {
        let rgb = self.resolved_top_bar_rgb();
        let bottom = [rgb[0], rgb[1], rgb[2], 1.0];
        let top = [rgb[0] * 0.5, rgb[1] * 0.5, rgb[2] * 0.5, 1.0];
        (top, bottom)
    }

    pub fn set_top_bar_rgb(&mut self, rgb: [f32; 3]) {
        self.top_color = PfaTopColor::Red;
        self.top_bar_rgb = rgb;
    }
}

impl SceneLayout {
    pub fn piano_height(&self) -> f32 {
        match &self.scene {
            SceneConfig::TwoD(scene) => scene
                .keyboard_height
                .resolve(self.viewport_width, self.viewport_height),
            SceneConfig::ThreeD(_) => 0.151,
        }
    }

    pub fn set_renderer_kind(&mut self, renderer: RendererKind) {
        self.scene = match renderer {
            RendererKind::Flat => SceneConfig::TwoD(TwoDSceneConfig {
                keyboard_height: KeyboardHeightSpec::default(),
                notes: NoteProjectorConfig::Flat(FlatNoteProjectorConfig::default()),
                keyboard: KeyboardProjectorConfig::Flat(FlatKeyboardProjectorConfig),
            }),
            RendererKind::Pfa => SceneConfig::TwoD(TwoDSceneConfig::default()),
            RendererKind::Miditrail => {
                SceneConfig::ThreeD(ThreeDSceneConfig::Miditrail(MiditrailSceneConfig {
                    box_notes: true,
                    ..MiditrailSceneConfig::default()
                }))
            }
        };
    }
}

impl NoteProjectorConfig {
    pub fn palette(&self) -> &NotePaletteConfig {
        match self {
            Self::Flat(config) => &config.palette,
            Self::Pfa(config) => &config.palette,
        }
    }
}

impl ThreeDSceneConfig {
    pub fn palette(&self) -> &NotePaletteConfig {
        match self {
            Self::Miditrail(config) => &config.palette,
        }
    }
}

const fn default_border_width() -> f32 {
    1.0
}

const fn default_top_bar_rgb() -> [f32; 3] {
    [0.585, 0.0392, 0.0249]
}

const fn default_miditrail_same_width_notes() -> bool {
    true
}

const fn default_miditrail_fov() -> f32 {
    std::f32::consts::PI / 3.0
}

const fn default_miditrail_view_height() -> f32 {
    0.5
}

const fn default_miditrail_view_offset() -> f32 {
    0.4
}

const fn default_miditrail_cam_ang() -> f32 {
    0.56
}

const fn default_miditrail_viewdist() -> f32 {
    14.0
}

const fn default_miditrail_viewback() -> f32 {
    0.2
}

const fn default_miditrail_note_down_speed() -> f32 {
    0.6
}

const fn default_miditrail_note_up_speed() -> f32 {
    0.2
}

const fn default_miditrail_show_keyboard() -> bool {
    true
}

const fn default_miditrail_tilt_keys() -> bool {
    true
}

const fn default_miditrail_aura_strength() -> f32 {
    2.0
}

const fn default_miditrail_aura_enabled() -> bool {
    true
}

const fn default_miditrail_notes_change_tint() -> bool {
    true
}

fn default_miditrail_aura_image() -> ProjectorImageConfig {
    ProjectorImageConfig::Builtin {
        name: "ring".to_string(),
    }
}
