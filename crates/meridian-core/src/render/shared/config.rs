//! Shared render schema.
//!
//! This file holds the render configuration types that are shared across the
//! CLI, UI, SDK, and core renderer.
//!
//! The main sections are:
//! - renderer kind and background selection
//! - 2D/3D/text scene configs
//! - projector/text overlay configs
//! - scene layout and display-time helpers
//!
//! The file is intentionally broad because these config types are serialized
//! together, but the sectioning below should make it easier to find a specific
//! knob without learning the entire render taxonomy first.

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
    Text,
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

// 3D scene configuration stays grouped with the `SceneConfig` enum because the
// enum is the public entry point while the concrete config remains a single
// implementation today.
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

// Text scenes share the same serialized boundary as the other scene configs,
// but the internal structure is intentionally kept separate from the 2D/3D
// projector settings above.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct TextSceneConfig {
    #[serde(default)]
    pub background: ProjectorBackgroundConfig,
    #[serde(default = "default_text_background_color")]
    pub background_color: String,
    #[serde(default = "default_text_default_style_name")]
    pub default_style: String,
    #[serde(default = "default_text_styles")]
    pub styles: Vec<TextStyleConfig>,
    #[serde(default = "default_text_overlays")]
    pub overlays: Vec<TextOverlayConfig>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TextAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Center,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TextAlignment {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TextValueSource {
    MidiName,
    RendererName,
    ViewportWidth,
    ViewportHeight,
    CurrentTimeSeconds,
    RemainingTimeSeconds,
    MidiLengthSeconds,
    CurrentTick,
    RemainingTick,
    MidiLengthTick,
    TotalNotes,
    PassedNotes,
    RemainingNotes,
    VisibleNotes,
    ActiveKeys,
    CurrentPolyphony,
    CurrentBpm,
    CurrentNps1s,
    CurrentNps2s,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum TextValueFormat {
    Raw,
    Integer,
    Decimal1,
    Decimal2,
    Decimal3,
    Clock,
    Seconds1,
    Seconds2,
    Bpm,
    Ticks,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct TextStyleConfig {
    #[serde(default = "default_text_style_name")]
    pub name: String,
    #[serde(default = "default_text_font_family")]
    pub font_family: String,
    #[serde(default = "default_text_pixel_scale")]
    pub font_size: u32,
    #[serde(default = "default_text_color")]
    pub color: String,
    #[serde(default = "default_text_line_spacing")]
    pub line_spacing: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
pub struct TextOverlayConfig {
    #[serde(default = "default_text_overlay_name")]
    pub name: String,
    #[serde(default)]
    pub anchor: TextAnchor,
    #[serde(default = "default_text_x")]
    pub x: f32,
    #[serde(default = "default_text_y")]
    pub y: f32,
    #[serde(default = "default_text_overlay_width")]
    pub width: f32,
    #[serde(default = "default_text_overlay_padding")]
    pub padding: f32,
    #[serde(default = "default_text_overlay_row_gap")]
    pub row_gap: f32,
    #[serde(default)]
    pub background_color: Option<String>,
    #[serde(default)]
    pub alignment: TextAlignment,
    #[serde(default = "default_text_default_style_name")]
    pub style: String,
    #[serde(default = "default_text_rows")]
    pub rows: Vec<TextRowConfig>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(tag = "row_type", rename_all = "snake_case")]
pub enum TextRowConfig {
    PlainText {
        text: String,
        #[serde(default)]
        style: Option<String>,
    },
    Metric {
        #[serde(default)]
        label: String,
        source: TextValueSource,
        #[serde(default)]
        format: TextValueFormat,
        #[serde(default)]
        prefix: String,
        #[serde(default)]
        suffix: String,
        #[serde(default)]
        style: Option<String>,
    },
    MetricPair {
        #[serde(default)]
        label: String,
        primary_source: TextValueSource,
        #[serde(default)]
        primary_format: TextValueFormat,
        secondary_source: TextValueSource,
        #[serde(default)]
        secondary_format: Option<TextValueFormat>,
        #[serde(default = "default_text_pair_separator")]
        separator: String,
        #[serde(default)]
        style: Option<String>,
    },
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
    Text(TextSceneConfig),
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

impl Default for TextSceneConfig {
    fn default() -> Self {
        Self {
            background: ProjectorBackgroundConfig::default(),
            background_color: default_text_background_color(),
            default_style: default_text_default_style_name(),
            styles: default_text_styles(),
            overlays: default_text_overlays(),
        }
    }
}

impl Default for TextAnchor {
    fn default() -> Self {
        Self::TopLeft
    }
}

impl Default for TextAlignment {
    fn default() -> Self {
        Self::Left
    }
}

impl Default for TextValueFormat {
    fn default() -> Self {
        Self::Raw
    }
}

impl Default for TextStyleConfig {
    fn default() -> Self {
        Self {
            name: default_text_style_name(),
            font_family: default_text_font_family(),
            font_size: default_text_pixel_scale(),
            color: default_text_color(),
            line_spacing: default_text_line_spacing(),
        }
    }
}

impl Default for TextOverlayConfig {
    fn default() -> Self {
        Self {
            name: default_text_overlay_name(),
            anchor: TextAnchor::default(),
            x: default_text_x(),
            y: default_text_y(),
            width: default_text_overlay_width(),
            padding: default_text_overlay_padding(),
            row_gap: default_text_overlay_row_gap(),
            background_color: None,
            alignment: TextAlignment::default(),
            style: default_text_default_style_name(),
            rows: default_text_rows(),
        }
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
            SceneConfig::Text(_) => 0.0,
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
            RendererKind::Text => SceneConfig::Text(TextSceneConfig {
                background,
                ..TextSceneConfig::default()
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
            Self::Text(config) => &config.background,
        }
    }

    pub fn background_mut(&mut self) -> &mut ProjectorBackgroundConfig {
        match self {
            Self::TwoD(config) => &mut config.background,
            Self::ThreeD(ThreeDSceneConfig::PianoTrailClassic(config)) => &mut config.background,
            Self::Text(config) => &mut config.background,
        }
    }

    pub fn renderer_kind(&self) -> RendererKind {
        match self {
            Self::TwoD(scene) => match &scene.notes {
                NoteProjectorConfig::Flat(_) => RendererKind::Flat,
                NoteProjectorConfig::Pfa(_) => RendererKind::Pfa,
            },
            Self::Text(_) => RendererKind::Text,
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
    let [r, g, b, _a] = parse_rgba_color(value)?;
    Some([r, g, b])
}

fn parse_rgba_color(value: &str) -> Option<[f32; 4]> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("transparent") {
        return Some([0.0, 0.0, 0.0, 0.0]);
    }

    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }

    match hex.len() {
        6 => Some([
            u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0,
            u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0,
            u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0,
            1.0,
        ]),
        8 => Some([
            u8::from_str_radix(&hex[0..2], 16).ok()? as f32 / 255.0,
            u8::from_str_radix(&hex[2..4], 16).ok()? as f32 / 255.0,
            u8::from_str_radix(&hex[4..6], 16).ok()? as f32 / 255.0,
            u8::from_str_radix(&hex[6..8], 16).ok()? as f32 / 255.0,
        ]),
        _ => None,
    }
}

impl TextSceneConfig {
    pub fn background_rgba(&self) -> [f32; 4] {
        parse_rgba_color(self.background_color.as_str()).unwrap_or([0.05, 0.08, 0.12, 1.0])
    }

    pub fn style_named(&self, name: &str) -> Option<&TextStyleConfig> {
        self.styles
            .iter()
            .find(|style| style.normalized_name() == name.trim())
    }

    pub fn default_style_config(&self) -> &TextStyleConfig {
        self.style_named(self.default_style.as_str())
            .or_else(|| self.styles.first())
            .expect("text scene should always contain at least one style")
    }
}

impl TextStyleConfig {
    pub fn normalized_name(&self) -> &str {
        let trimmed = self.name.trim();
        if trimmed.is_empty() {
            "body"
        } else {
            trimmed
        }
    }

    pub fn normalized_font_family(&self) -> &str {
        let trimmed = self.font_family.trim();
        if trimmed.is_empty() {
            "sans-serif"
        } else {
            trimmed
        }
    }

    pub fn rgba(&self) -> [f32; 4] {
        parse_rgba_color(self.color.as_str()).unwrap_or([0.9, 0.94, 0.98, 1.0])
    }

    pub fn resolved_line_spacing(&self) -> f32 {
        self.line_spacing.max(0.0)
    }

    pub fn resolved_font_size(&self) -> u32 {
        self.font_size.max(1)
    }
}

impl TextOverlayConfig {
    pub fn resolved_width(&self) -> f32 {
        self.width.clamp(0.05, 1.0)
    }

    pub fn resolved_style_name(&self) -> &str {
        let trimmed = self.style.trim();
        if trimmed.is_empty() {
            "body"
        } else {
            trimmed
        }
    }

    pub fn background_rgba(&self) -> Option<[f32; 4]> {
        self.background_color
            .as_ref()
            .and_then(|value| parse_rgba_color(value.as_str()))
    }
}

impl TextRowConfig {
    pub fn style_name(&self) -> Option<&str> {
        match self {
            Self::PlainText { style, .. }
            | Self::Metric { style, .. }
            | Self::MetricPair { style, .. } => style.as_deref(),
        }
    }
}

fn default_text_styles() -> Vec<TextStyleConfig> {
    vec![
        TextStyleConfig {
            name: "title".to_string(),
            font_family: default_text_font_family(),
            font_size: 64,
            color: "#F6F1E7".to_string(),
            line_spacing: 14.0,
        },
        TextStyleConfig {
            name: "body".to_string(),
            font_family: default_text_font_family(),
            font_size: 42,
            color: default_text_color(),
            line_spacing: 10.0,
        },
        TextStyleConfig {
            name: "mono".to_string(),
            font_family: "monospace".to_string(),
            font_size: 34,
            color: "#BCE7D0".to_string(),
            line_spacing: 8.0,
        },
    ]
}

fn default_text_rows() -> Vec<TextRowConfig> {
    vec![
        TextRowConfig::PlainText {
            text: "MERIDIAN".to_string(),
            style: Some("title".to_string()),
        },
        TextRowConfig::PlainText {
            text: "Structured text scene".to_string(),
            style: None,
        },
        TextRowConfig::PlainText {
            text: "Time {{time.current|clock}}".to_string(),
            style: Some("mono".to_string()),
        },
    ]
}

fn default_text_overlays() -> Vec<TextOverlayConfig> {
    vec![TextOverlayConfig::default()]
}

fn default_text_overlay_name() -> String {
    "Overlay 1".to_string()
}

fn default_text_pair_separator() -> String {
    " / ".to_string()
}
