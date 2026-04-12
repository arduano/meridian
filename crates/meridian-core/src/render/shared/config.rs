use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{NotePaletteConfig, ProjectorImageConfig};

mod defaults;
use defaults::*;

pub const DEFAULT_PFA_KEYBOARD_ASPECT_RATIO: f32 = 0.084_937_5;
pub const PFA_RED_TOP_BAR_COLOR: &str = "#950A06";
pub const PFA_BLUE_TOP_BAR_COLOR: &str = "#0A0695";
pub const PFA_GREEN_TOP_BAR_COLOR: &str = "#06950A";

#[derive(Clone, Copy, Debug, Eq, PartialEq, clap::ValueEnum, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum RendererKind {
    Flat,
    Pfa,
    #[serde(rename = "piano_trail_classic")]
    #[value(name = "piano-trail-classic")]
    PianoTrailClassic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ProjectorBackgroundScalingMode {
    Stretch,
    Cover,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum ProjectorBackgroundConfig {
    None,
    PngFile {
        path: String,
        #[serde(default)]
        scaling: ProjectorBackgroundScalingMode,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct FlatNoteProjectorConfig {
    #[serde(default)]
    pub palette: NotePaletteConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct PfaNoteProjectorConfig {
    #[serde(default)]
    pub same_width_notes: bool,
    #[serde(default = "default_border_width")]
    pub border_width: f32,
    #[serde(default)]
    pub palette: NotePaletteConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct FlatKeyboardProjectorConfig;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct PfaKeyboardProjectorConfig {
    #[serde(default)]
    pub same_width_notes: bool,
    #[serde(default)]
    pub middle_c: bool,
    #[serde(default = "default_top_bar_color")]
    pub top_bar_color: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "projector", rename_all = "snake_case")]
pub enum NoteProjectorConfig {
    Flat(FlatNoteProjectorConfig),
    Pfa(PfaNoteProjectorConfig),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "projector", rename_all = "snake_case")]
pub enum KeyboardProjectorConfig {
    Flat(FlatKeyboardProjectorConfig),
    Pfa(PfaKeyboardProjectorConfig),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum KeyboardHeightSpec {
    ScreenPercent { height: f32 },
    AspectRatio { ratio: f32 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct TwoDSceneConfig {
    #[serde(default)]
    pub background: ProjectorBackgroundConfig,
    #[serde(default)]
    pub keyboard_height: KeyboardHeightSpec,
    #[serde(default)]
    pub notes: NoteProjectorConfig,
    #[serde(default)]
    pub keyboard: KeyboardProjectorConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct PianoTrailClassicSceneConfig {
    #[serde(default)]
    pub background: ProjectorBackgroundConfig,
    #[serde(default = "default_piano_trail_classic_same_width_notes")]
    pub same_width_notes: bool,
    #[serde(default = "default_piano_trail_classic_fov")]
    pub fov: f32,
    #[serde(default = "default_piano_trail_classic_view_height")]
    pub view_height: f32,
    #[serde(default = "default_piano_trail_classic_view_offset")]
    pub view_offset: f32,
    #[serde(default)]
    pub view_pan: f32,
    #[serde(default = "default_piano_trail_classic_cam_ang")]
    pub cam_ang: f32,
    #[serde(default)]
    pub cam_rot: f32,
    #[serde(default)]
    pub cam_spin: f32,
    #[serde(default = "default_piano_trail_classic_viewdist")]
    pub viewdist: f32,
    #[serde(default = "default_piano_trail_classic_viewback")]
    pub viewback: f32,
    #[serde(default)]
    pub vertical_notes: bool,
    #[serde(default = "default_piano_trail_classic_note_down_speed")]
    pub note_down_speed: f32,
    #[serde(default = "default_piano_trail_classic_note_up_speed")]
    pub note_up_speed: f32,
    #[serde(default)]
    pub box_notes: bool,
    #[serde(default)]
    pub light_shade: bool,
    #[serde(default = "default_piano_trail_classic_show_keyboard")]
    pub show_keyboard: bool,
    #[serde(default = "default_piano_trail_classic_tilt_keys")]
    pub tilt_keys: bool,
    #[serde(default)]
    pub eat_notes: bool,
    #[serde(default = "default_piano_trail_classic_aura_strength")]
    pub aura_strength: f32,
    #[serde(default = "default_piano_trail_classic_aura_enabled")]
    pub aura_enabled: bool,
    #[serde(default)]
    pub notes_change_size: bool,
    #[serde(default = "default_piano_trail_classic_notes_change_tint")]
    pub notes_change_tint: bool,
    #[serde(default)]
    pub use_vel: bool,
    #[serde(default)]
    pub palette: NotePaletteConfig,
    #[serde(default = "default_piano_trail_classic_aura_image")]
    pub aura_image: ProjectorImageConfig,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "projector", rename_all = "snake_case")]
pub enum ThreeDSceneConfig {
    #[serde(rename = "piano_trail_classic")]
    PianoTrailClassic(PianoTrailClassicSceneConfig),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum DisplayTimeSpace {
    Time,
    Tick,
}

impl Default for DisplayTimeSpace {
    fn default() -> Self {
        Self::Time
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct SceneLayout {
    pub scene: SceneConfig,
    pub view_range: f64,
    #[serde(default)]
    pub time_space: DisplayTimeSpace,
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
            top_bar_color: default_top_bar_color(),
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
            background: ProjectorBackgroundConfig::default(),
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
            view_range: 0.5,
            time_space: DisplayTimeSpace::Time,
            first_key: 0,
            last_key: 127,
            viewport_width: 1280,
            viewport_height: 720,
        }
    }
}

impl Default for PianoTrailClassicSceneConfig {
    fn default() -> Self {
        Self {
            background: ProjectorBackgroundConfig::default(),
            same_width_notes: default_piano_trail_classic_same_width_notes(),
            fov: default_piano_trail_classic_fov(),
            view_height: default_piano_trail_classic_view_height(),
            view_offset: default_piano_trail_classic_view_offset(),
            view_pan: 0.0,
            cam_ang: default_piano_trail_classic_cam_ang(),
            cam_rot: 0.0,
            cam_spin: 0.0,
            viewdist: default_piano_trail_classic_viewdist(),
            viewback: default_piano_trail_classic_viewback(),
            vertical_notes: false,
            note_down_speed: default_piano_trail_classic_note_down_speed(),
            note_up_speed: default_piano_trail_classic_note_up_speed(),
            box_notes: false,
            light_shade: false,
            show_keyboard: default_piano_trail_classic_show_keyboard(),
            tilt_keys: default_piano_trail_classic_tilt_keys(),
            eat_notes: false,
            aura_strength: default_piano_trail_classic_aura_strength(),
            aura_enabled: default_piano_trail_classic_aura_enabled(),
            notes_change_size: false,
            notes_change_tint: default_piano_trail_classic_notes_change_tint(),
            use_vel: false,
            palette: NotePaletteConfig::default(),
            aura_image: default_piano_trail_classic_aura_image(),
        }
    }
}

impl Default for ThreeDSceneConfig {
    fn default() -> Self {
        Self::PianoTrailClassic(PianoTrailClassicSceneConfig::default())
    }
}

impl Default for ProjectorBackgroundScalingMode {
    fn default() -> Self {
        Self::Stretch
    }
}

impl Default for ProjectorBackgroundConfig {
    fn default() -> Self {
        Self::None
    }
}

impl PfaKeyboardProjectorConfig {
    pub fn resolved_top_bar_rgb(&self) -> [f32; 3] {
        parse_hex_color(self.top_bar_color.as_str()).unwrap_or_else(default_top_bar_rgb)
    }

    pub fn resolved_top_bar_gradient(&self) -> ([f32; 4], [f32; 4]) {
        let rgb = self.resolved_top_bar_rgb();
        let bottom = [rgb[0], rgb[1], rgb[2], 1.0];
        let top = [rgb[0] * 0.5, rgb[1] * 0.5, rgb[2] * 0.5, 1.0];
        (top, bottom)
    }

    pub fn set_top_bar_color(&mut self, value: &str) -> bool {
        let Some(color) = normalize_top_bar_color(value) else {
            return false;
        };
        self.top_bar_color = color;
        true
    }

    pub fn top_bar_preset_name(&self) -> Option<&'static str> {
        match self.top_bar_color.as_str() {
            PFA_RED_TOP_BAR_COLOR => Some("red"),
            PFA_BLUE_TOP_BAR_COLOR => Some("blue"),
            PFA_GREEN_TOP_BAR_COLOR => Some("green"),
            _ => None,
        }
    }

    pub fn normalize_top_bar_color(value: &str) -> Option<String> {
        normalize_top_bar_color(value)
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
        let background = self.scene.background().clone();
        self.scene = match renderer {
            RendererKind::Flat => SceneConfig::TwoD(TwoDSceneConfig {
                background,
                keyboard_height: KeyboardHeightSpec::default(),
                notes: NoteProjectorConfig::Flat(FlatNoteProjectorConfig::default()),
                keyboard: KeyboardProjectorConfig::Flat(FlatKeyboardProjectorConfig),
            }),
            RendererKind::Pfa => SceneConfig::TwoD(TwoDSceneConfig {
                background,
                ..TwoDSceneConfig::default()
            }),
            RendererKind::PianoTrailClassic => SceneConfig::ThreeD(
                ThreeDSceneConfig::PianoTrailClassic(PianoTrailClassicSceneConfig {
                    background,
                    box_notes: true,
                    ..PianoTrailClassicSceneConfig::default()
                }),
            ),
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

impl SceneConfig {
    pub fn background(&self) -> &ProjectorBackgroundConfig {
        match self {
            Self::TwoD(config) => &config.background,
            Self::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => &config.background,
        }
    }

    pub fn background_mut(&mut self) -> &mut ProjectorBackgroundConfig {
        match self {
            Self::TwoD(config) => &mut config.background,
            Self::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => &mut config.background,
        }
    }

    pub fn renderer_kind(&self) -> RendererKind {
        match self {
            Self::TwoD(scene) => match &scene.notes {
                NoteProjectorConfig::Flat(_) => RendererKind::Flat,
                NoteProjectorConfig::Pfa(_) => RendererKind::Pfa,
            },
            Self::ThreeD(ThreeDSceneConfig::PianoTrailClassic(_)) => {
                RendererKind::PianoTrailClassic
            }
        }
    }
}

impl ThreeDSceneConfig {
    pub fn palette(&self) -> &NotePaletteConfig {
        match self {
            Self::PianoTrailClassic(config) => &config.palette,
        }
    }
}

fn normalize_top_bar_color(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() != 6 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", hex.to_ascii_uppercase()))
}

fn parse_hex_color(value: &str) -> Option<[f32; 3]> {
    let normalized = normalize_top_bar_color(value)?;
    let hex = &normalized[1..];
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0,
        u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0,
        u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0,
    ])
}
