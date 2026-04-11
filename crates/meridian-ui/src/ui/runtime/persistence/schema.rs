use std::path::PathBuf;

use meridian_core::{
    audio::{AudioBackend, AudioConfig},
    midi::{MidiFileProcessingConfig, QuantizeTool},
    render::{DisplayTimeSpace, SceneConfig, SceneLayout},
};
use serde::{Deserialize, Serialize};

pub(in super::super) const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(in super::super) struct UiConfigFile {
    pub version: u32,
    pub preferences: UiPreferences,
}

impl Default for UiConfigFile {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            preferences: UiPreferences::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(in super::super) struct UiPreferences {
    pub audio: AudioConfig,
    pub scene: SceneConfig,
    pub view_range: f64,
    pub time_space: DisplayTimeSpace,
    pub first_key: u8,
    pub last_key: u8,
    pub active_profile: i32,
    pub export: ExportPreferences,
    pub modify: ModifyPreferences,
    pub merge: MergePreferences,
    pub window: WindowPreferences,
    pub last_palette_png: Option<PathBuf>,
    pub last_background_png: Option<String>,
    pub last_aura_png: Option<String>,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            audio: AudioConfig {
                backend: AudioBackend::Xsynth,
                ..AudioConfig::default()
            },
            scene: SceneLayout::default().scene,
            view_range: 0.5,
            time_space: DisplayTimeSpace::Time,
            first_key: 0,
            last_key: 127,
            active_profile: 0,
            export: ExportPreferences::default(),
            modify: ModifyPreferences::default(),
            merge: MergePreferences::default(),
            window: WindowPreferences::default(),
            last_palette_png: None,
            last_background_png: None,
            last_aura_png: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(in super::super) struct ExportPreferences {
    pub mode_text: String,
    pub video_resolution_text: String,
    pub video_fps_text: String,
    pub audio_format_text: String,
    pub audio_sample_rate_text: String,
    pub audio_channel_count_text: String,
    pub video_ffmpeg_args_text: String,
    pub audio_ffmpeg_args_text: String,
    pub video_codec_text: String,
    pub video_crf_text: String,
    pub video_preset_text: String,
    pub video_pix_fmt_text: String,
    pub video_rgb_mode_text: String,
    pub audio_bitrate_text: String,
    pub export_alpha_mask: bool,
    pub use_limiter: bool,
    pub open_after_export: bool,
}

impl Default for ExportPreferences {
    fn default() -> Self {
        Self {
            mode_text: "video_audio".into(),
            video_resolution_text: "1920x1080".into(),
            video_fps_text: "60".into(),
            audio_format_text: "wav".into(),
            audio_sample_rate_text: "48000".into(),
            audio_channel_count_text: "stereo".into(),
            video_ffmpeg_args_text: String::new(),
            audio_ffmpeg_args_text: String::new(),
            video_codec_text: "libx264".into(),
            video_crf_text: "18".into(),
            video_preset_text: "medium".into(),
            video_pix_fmt_text: "yuv420p".into(),
            video_rgb_mode_text: "premultiplied".into(),
            audio_bitrate_text: "192k".into(),
            export_alpha_mask: false,
            use_limiter: true,
            open_after_export: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(in super::super) struct ModifyPreferences {
    pub pass_key_text: String,
    pub config_text: Option<String>,
    pub last_valid_config_text: Option<String>,
}

impl Default for ModifyPreferences {
    fn default() -> Self {
        Self {
            pass_key_text: "quantize".into(),
            config_text: Some(default_modify_config_text()),
            last_valid_config_text: Some(default_modify_config_text()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub(in super::super) struct MergePreferences {
    pub layout_text: String,
    pub metadata_mode_text: String,
    pub ppq_override_text: String,
}

impl Default for MergePreferences {
    fn default() -> Self {
        Self {
            layout_text: "append_tracks".into(),
            metadata_mode_text: "keep".into(),
            ppq_override_text: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(default)]
pub(in super::super) struct WindowPreferences {
    pub position: Option<WindowPosition>,
    pub size: Option<WindowSize>,
    pub maximized: bool,
    pub fullscreen: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct WindowPosition {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub(in super::super) struct WindowSize {
    pub width: u32,
    pub height: u32,
}

fn default_modify_config_text() -> String {
    serde_json::to_string_pretty(&MidiFileProcessingConfig {
        tool: meridian_core::midi::MidiModifierTool::Quantize(QuantizeTool {
            rounding_ticks: 120,
            mode: meridian_core::midi::QuantizeMode::NoteStartOnly,
        }),
    })
    .unwrap_or_default()
}
