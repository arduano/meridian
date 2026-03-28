use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::render::{ProjectedScene, SceneConfig, SceneLayout};

pub const PROTOCOL_VERSION: u32 = 1;

pub const fn protocol_version() -> u32 {
    PROTOCOL_VERSION
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreCommand {
    GetState,
    LoadMidi {
        path: PathBuf,
    },
    SetTime {
        time: f64,
    },
    StepTime {
        delta: f64,
    },
    SetPlaying {
        playing: bool,
    },
    TogglePlaying,
    SetSceneConfig {
        scene: SceneConfig,
    },
    SetViewRange {
        seconds: f64,
    },
    SetKeyRange {
        first_key: u8,
        last_key: u8,
    },
    SetViewport {
        width: u32,
        height: u32,
    },
    RenderFrame {
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    },
    SaveFrame {
        output: PathBuf,
        format: Option<ImageOutputFormat>,
        viewport_width: Option<u32>,
        viewport_height: Option<u32>,
    },
    Shutdown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum ImageOutputFormat {
    Ppm,
    Png,
    Rgba,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    pub midi_path: Option<PathBuf>,
    pub midi_loaded: bool,
    pub scene: SceneConfig,
    pub current_time: f64,
    pub playing: bool,
    pub midi_length: f64,
    pub total_notes: u64,
    pub view_range: f64,
    pub first_key: u8,
    pub last_key: u8,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoreErrorCode {
    InvalidJson,
    InvalidProtocolVersion,
    InvalidCommand,
    InvalidViewport,
    InvalidLayout,
    NoMidiLoaded,
    UnsupportedFormat,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreEvent {
    StateSnapshot {
        state: StateSnapshot,
    },
    MidiLoaded {
        path: PathBuf,
        state: StateSnapshot,
    },
    FrameProjected {
        state: StateSnapshot,
        layout: SceneLayout,
        stats: FrameStats,
    },
    FrameSaved {
        output: PathBuf,
        format: ImageOutputFormat,
        state: StateSnapshot,
        stats: FrameStats,
        bytes_written: u64,
    },
    Error {
        code: CoreErrorCode,
        message: String,
    },
    ShutdownComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRequest {
    #[serde(default = "protocol_version")]
    pub protocol_version: u32,
    pub id: Option<u64>,
    pub command: CoreCommand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonResponse {
    #[serde(default = "protocol_version")]
    pub protocol_version: u32,
    pub id: Option<u64>,
    pub events: Vec<CoreEvent>,
}
