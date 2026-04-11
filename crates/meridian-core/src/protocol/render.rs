use std::path::{Path, PathBuf};

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::render::{DisplayTimeSpace, ProjectedScene, SceneConfig, SceneLayout};

use super::{
    ids::{AudioRenderJobId, VideoRenderJobId},
    state::StateSnapshot,
};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct FrameStats {
    pub visible_notes: usize,
    pub active_keys: usize,
    pub note_quads: usize,
    pub keyboard_quads: usize,
    pub total_quads: usize,
    pub total_vertices: usize,
}

impl FrameStats {
    pub fn from_scene(scene: &ProjectedScene) -> Self {
        Self {
            visible_notes: scene.visible_notes,
            active_keys: scene.active_keys,
            note_quads: scene.note_quads,
            keyboard_quads: scene.keyboard_quads,
            total_quads: scene.total_quads(),
            total_vertices: scene.total_vertices(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RenderedFrame {
    pub state: StateSnapshot,
    pub layout: SceneLayout,
    pub stats: FrameStats,
    pub scene: ProjectedScene,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum FrameColorMode {
    #[default]
    Premultiplied,
    Straight,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct ImageExportConfig {
    #[serde(default)]
    pub color_mode: FrameColorMode,
    #[serde(default)]
    pub export_premultiplied_rgb: bool,
    #[serde(default)]
    pub export_straight_rgb: bool,
    #[serde(default)]
    pub export_alpha_mask: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct ImageExportArtifacts {
    #[serde(default)]
    pub premultiplied_rgb: Option<PathBuf>,
    #[serde(default)]
    pub straight_rgb: Option<PathBuf>,
    #[serde(default)]
    pub alpha_mask: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct VideoExportConfig {
    #[serde(default)]
    pub color_mode: FrameColorMode,
    #[serde(default)]
    pub export_alpha_mask: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct VideoExportArtifacts {
    #[serde(default)]
    pub alpha_mask: Option<PathBuf>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct VideoAudioProgress {
    #[serde(default)]
    pub total_events: usize,
    #[serde(default)]
    pub event_index: usize,
    #[serde(default)]
    pub rendered_seconds: f64,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AudioOutputFormat {
    #[default]
    Wav,
    Flac,
    Mp3,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, TS, ValueEnum, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoOutputContainer {
    #[default]
    Mp4,
    Mkv,
}

impl VideoOutputContainer {
    pub fn extension(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "mkv",
        }
    }

    pub fn ffmpeg_format_name(self) -> &'static str {
        match self {
            Self::Mp4 => "mp4",
            Self::Mkv => "matroska",
        }
    }

    pub fn matches_path(self, path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case(self.extension()))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
pub struct VideoAudioConfig {
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub use_limiter: Option<bool>,
    #[serde(default)]
    pub soundfonts: Vec<PathBuf>,
    #[serde(default)]
    pub ffmpeg_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRenderConfig {
    pub midi_path: Option<PathBuf>,
    pub output: PathBuf,
    pub container: VideoOutputContainer,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
    pub scene: Option<SceneConfig>,
    pub view_range: Option<f64>,
    #[serde(default)]
    pub time_space: Option<DisplayTimeSpace>,
    pub first_key: Option<u8>,
    pub last_key: Option<u8>,
    #[serde(default)]
    pub ffmpeg_args: Vec<String>,
    #[serde(default)]
    pub export: VideoExportConfig,
    #[serde(default)]
    pub audio: Option<VideoAudioConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum AudioRenderStatus {
    Idle,
    Running {
        job_id: AudioRenderJobId,
        output: PathBuf,
        total_events: usize,
        event_index: usize,
        time_seconds: f64,
        rendered_seconds: f64,
        frames_written: u64,
    },
    Cancelling {
        job_id: AudioRenderJobId,
        output: PathBuf,
        total_events: usize,
        event_index: usize,
        time_seconds: f64,
        rendered_seconds: f64,
        frames_written: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VideoRenderEvent {
    RenderStarted {
        job_id: VideoRenderJobId,
        midi: Option<PathBuf>,
        output: PathBuf,
        container: VideoOutputContainer,
        exports: VideoExportArtifacts,
        fps: f64,
        width: u32,
        height: u32,
        total_frames: u64,
        duration_seconds: f64,
        ffmpeg_command: Vec<String>,
        #[serde(default)]
        alpha_ffmpeg_command: Option<Vec<String>>,
        #[serde(default)]
        audio_progress: Option<VideoAudioProgress>,
    },
    RenderProgress {
        job_id: VideoRenderJobId,
        frame_index: u64,
        total_frames: u64,
        current_time: f64,
        elapsed_seconds: f64,
        average_fps: f64,
        #[serde(default)]
        audio_progress: Option<VideoAudioProgress>,
    },
    RenderCancelled {
        job_id: VideoRenderJobId,
        frame_index: u64,
        total_frames: u64,
        elapsed_seconds: f64,
    },
    RenderFinished {
        job_id: VideoRenderJobId,
        total_frames: u64,
        elapsed_seconds: f64,
        average_fps: f64,
        output: PathBuf,
        container: VideoOutputContainer,
        exports: VideoExportArtifacts,
    },
    RenderFailed {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VideoRenderStatus {
    Idle,
    Running {
        job_id: VideoRenderJobId,
        output: PathBuf,
        container: VideoOutputContainer,
        fps: f64,
        width: u32,
        height: u32,
        total_frames: u64,
        frame_index: u64,
        current_time: f64,
        elapsed_seconds: f64,
        #[serde(default)]
        audio_progress: Option<VideoAudioProgress>,
    },
    Cancelling {
        job_id: VideoRenderJobId,
        output: PathBuf,
        container: VideoOutputContainer,
        fps: f64,
        width: u32,
        height: u32,
        total_frames: u64,
        frame_index: u64,
        current_time: f64,
        elapsed_seconds: f64,
        #[serde(default)]
        audio_progress: Option<VideoAudioProgress>,
    },
}
