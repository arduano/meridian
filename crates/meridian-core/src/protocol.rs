use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::{
    audio::{AudioConfig, AudioRenderConfig, AudioRenderEvent, AudioStatus},
    render::{ProjectedScene, SceneConfig, SceneLayout},
};

pub const PROTOCOL_VERSION: u32 = 1;

pub const fn protocol_version() -> u32 {
    PROTOCOL_VERSION
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ParsedMidiId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DisplayCacheId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AudioCacheId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreCommand {
    GetState,
    LoadParsedMidi {
        path: PathBuf,
    },
    BuildDisplayCache {
        parsed_midi_id: ParsedMidiId,
    },
    BuildAudioCache {
        parsed_midi_id: ParsedMidiId,
    },
    AttachDisplayCache {
        display_cache_id: DisplayCacheId,
    },
    AttachAudioCache {
        audio_cache_id: AudioCacheId,
    },
    LoadMidi {
        path: PathBuf,
    },
    SetAudioConfig {
        config: AudioConfig,
    },
    GetAudioStatus,
    StartRenderAudio {
        config: AudioRenderConfig,
    },
    CancelRenderAudio,
    GetRenderAudioStatus,
    SetTime {
        time: f64,
    },
    TickProjectorPhysics {
        delta_seconds: f64,
    },
    ResetProjectorPhysics,
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
    StartRenderVideo {
        config: VideoRenderConfig,
    },
    CancelRenderVideo,
    GetRenderVideoStatus,
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
    pub active_parsed_midi_id: Option<ParsedMidiId>,
    pub active_display_cache_id: Option<DisplayCacheId>,
    pub active_audio_cache_id: Option<AudioCacheId>,
    pub midi_path: Option<PathBuf>,
    pub midi_loaded: bool,
    pub audio: AudioConfig,
    pub audio_status: AudioStatus,
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
        output: PathBuf,
        total_events: usize,
        event_index: usize,
        time_seconds: f64,
        rendered_seconds: f64,
        frames_written: u64,
    },
    Cancelling {
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
        frame_index: u64,
        total_frames: u64,
        current_time: f64,
        elapsed_seconds: f64,
        average_fps: f64,
    },
    RenderCancelled {
        frame_index: u64,
        total_frames: u64,
        elapsed_seconds: f64,
    },
    RenderFinished {
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
    ParsedMidiLoaded {
        parsed_midi_id: ParsedMidiId,
        path: PathBuf,
    },
    DisplayCacheBuilt {
        parsed_midi_id: ParsedMidiId,
        display_cache_id: DisplayCacheId,
        midi_length: f64,
        total_notes: u64,
        track_count: usize,
    },
    AudioCacheBuilt {
        parsed_midi_id: ParsedMidiId,
        audio_cache_id: AudioCacheId,
        total_events: usize,
    },
    DisplayCacheAttached {
        display_cache_id: DisplayCacheId,
        state: StateSnapshot,
    },
    AudioCacheAttached {
        audio_cache_id: AudioCacheId,
        state: StateSnapshot,
    },
    StateSnapshot {
        state: StateSnapshot,
    },
    MidiLoaded {
        path: PathBuf,
        state: StateSnapshot,
    },
    AudioStatus {
        status: AudioStatus,
    },
    AudioRender {
        event: AudioRenderEvent,
    },
    AudioRenderStatus {
        status: AudioRenderStatus,
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
    VideoRender {
        event: VideoRenderEvent,
    },
    VideoRenderStatus {
        status: VideoRenderStatus,
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
