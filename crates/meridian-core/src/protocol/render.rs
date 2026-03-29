use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::render::{ProjectedScene, SceneConfig, SceneLayout};

use super::{
    ids::{AudioRenderJobId, VideoRenderJobId},
    state::StateSnapshot,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRenderConfig {
    pub midi_path: Option<PathBuf>,
    pub output: PathBuf,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
    pub scene: Option<SceneConfig>,
    pub view_range: Option<f64>,
    pub first_key: Option<u8>,
    pub last_key: Option<u8>,
    #[serde(default)]
    pub ffmpeg_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum VideoRenderEvent {
    RenderStarted {
        job_id: VideoRenderJobId,
        midi: Option<PathBuf>,
        output: PathBuf,
        fps: f64,
        width: u32,
        height: u32,
        total_frames: u64,
        duration_seconds: f64,
        ffmpeg_command: Vec<String>,
    },
    RenderProgress {
        job_id: VideoRenderJobId,
        frame_index: u64,
        total_frames: u64,
        current_time: f64,
        elapsed_seconds: f64,
        average_fps: f64,
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
    },
    RenderFailed {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum VideoRenderStatus {
    Idle,
    Running {
        job_id: VideoRenderJobId,
        output: PathBuf,
        fps: f64,
        width: u32,
        height: u32,
        total_frames: u64,
        frame_index: u64,
        current_time: f64,
        elapsed_seconds: f64,
    },
    Cancelling {
        job_id: VideoRenderJobId,
        output: PathBuf,
        fps: f64,
        width: u32,
        height: u32,
        total_frames: u64,
        frame_index: u64,
        current_time: f64,
        elapsed_seconds: f64,
    },
}
