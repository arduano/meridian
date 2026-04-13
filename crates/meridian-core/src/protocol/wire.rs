use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    audio::AudioRenderEvent,
    midi::{MidiFileProcessingConfig, MidiFilesMergeConfig},
    protocol::{
        AudioRenderStatus, CoreCommand, CoreErrorCode, CoreEvent, ImageExportConfig,
        MidiAnalysisData, MidiAnalysisJobEvent, MidiAnalysisJobStatus, MidiAnalysisKind,
        MidiFileInspection, MidiProcessEvent, MidiProcessJobId, MidiProcessStatus,
        PROTOCOL_VERSION, ParsedMidiId, VideoAudioConfig, VideoExportConfig, VideoOutputContainer,
        VideoRenderEvent, VideoRenderStatus,
    },
    render::{DisplayTimeSpace, RendererKind, SceneConfig, SceneLayout},
};

mod conversions;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProtocolAudioRenderConfig {
    pub midi_path: PathBuf,
    pub output: PathBuf,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub use_limiter: Option<bool>,
    #[serde(default)]
    pub format: crate::protocol::AudioOutputFormat,
    #[serde(default)]
    pub ffmpeg_args: Vec<String>,
    pub soundfonts: Vec<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProtocolStateSnapshot {
    pub midi_path: Option<PathBuf>,
    pub current_time: f64,
    pub playing: bool,
    pub scene: SceneConfig,
    pub view_range: f64,
    pub time_space: DisplayTimeSpace,
    pub first_key: u8,
    pub last_key: u8,
    pub viewport_width: u32,
    pub viewport_height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProtocolVideoRenderConfig {
    pub midi_path: PathBuf,
    pub output: PathBuf,
    #[serde(default)]
    pub container: VideoOutputContainer,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub renderer: Option<RendererKind>,
    #[serde(default)]
    pub scene: Option<SceneConfig>,
    pub view_range: Option<f64>,
    #[serde(default)]
    pub time_space: Option<DisplayTimeSpace>,
    #[serde(default)]
    pub start_time: Option<f64>,
    #[serde(default)]
    pub end_time: Option<f64>,
    pub first_key: Option<u8>,
    pub last_key: Option<u8>,
    #[serde(default)]
    pub ffmpeg_args: Vec<String>,
    #[serde(default)]
    pub export: VideoExportConfig,
    #[serde(default)]
    pub audio: Option<VideoAudioConfig>,
}

#[expect(
    clippy::large_enum_variant,
    reason = "Protocol commands stay unboxed at the wire boundary so serde/TS bindings remain straightforward."
)]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolCommand {
    LoadParsedMidi {
        path: PathBuf,
    },
    InspectMidiFiles {
        paths: Vec<PathBuf>,
    },
    LoadMidi {
        path: PathBuf,
    },
    LoadAudioMidi {
        path: PathBuf,
    },
    StartMidiAnalysisJob {
        parsed_midi_id: ParsedMidiId,
        #[serde(default)]
        kinds: Vec<MidiAnalysisKind>,
        #[serde(default)]
        bucket_count: Option<usize>,
    },
    StartProcessMidiFile {
        input: PathBuf,
        output: PathBuf,
        config: MidiFileProcessingConfig,
    },
    MergeMidiFiles {
        inputs: Vec<PathBuf>,
        output: PathBuf,
        config: MidiFilesMergeConfig,
    },
    CancelMidiFileProcess,
    GetMidiFileProcessStatus,
    StartRenderAudio {
        config: ProtocolAudioRenderConfig,
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
        #[serde(default)]
        time_space: Option<DisplayTimeSpace>,
    },
    SetKeyRange {
        first_key: u8,
        last_key: u8,
    },
    SetViewport {
        width: u32,
        height: u32,
    },
    SaveFrame {
        output: PathBuf,
        #[serde(default)]
        format: Option<crate::protocol::ImageOutputFormat>,
        #[serde(default)]
        viewport_width: Option<u32>,
        #[serde(default)]
        viewport_height: Option<u32>,
        #[serde(default)]
        export: ImageExportConfig,
    },
    StartRenderVideo {
        config: ProtocolVideoRenderConfig,
    },
    CancelRenderVideo,
    GetRenderVideoStatus,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolEvent {
    ParsedMidiLoaded {
        parsed_midi_id: ParsedMidiId,
        path: PathBuf,
    },
    MidiFilesInspected {
        inspections: Vec<MidiFileInspection>,
    },
    StateSnapshot {
        state: ProtocolStateSnapshot,
    },
    MidiLoaded {
        path: PathBuf,
    },
    MidiFileProcessed {
        input: PathBuf,
        output: PathBuf,
        output_track_count: usize,
        output_ppq: u16,
        total_events: usize,
    },
    MidiFilesMerged {
        output: PathBuf,
        input_count: usize,
        output_track_count: usize,
        output_ppq: u16,
        total_events: usize,
    },
    MidiAnalysisJob {
        event: MidiAnalysisJobEvent,
    },
    MidiAnalysisJobStatus {
        status: MidiAnalysisJobStatus,
    },
    MidiProcess {
        event: MidiProcessEvent,
    },
    MidiProcessStatus {
        status: MidiProcessStatus,
    },
    AudioRender {
        event: AudioRenderEvent,
    },
    AudioRenderStatus {
        status: AudioRenderStatus,
    },
    FrameSaved {
        output: PathBuf,
        format: crate::protocol::ImageOutputFormat,
        stats: crate::protocol::FrameStats,
        bytes_written: u64,
        exports: crate::protocol::ImageExportArtifacts,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedProtocolEvent;

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProtocolRequest {
    pub protocol_version: u32,
    pub id: Option<u64>,
    pub command: ProtocolCommand,
}

impl Default for ProtocolRequest {
    fn default() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            id: None,
            command: ProtocolCommand::GetMidiFileProcessStatus,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProtocolResponse {
    pub protocol_version: u32,
    pub id: Option<u64>,
    pub events: Vec<ProtocolEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProtocolSchemaDemo {
    pub protocol_version: u32,
    pub example_request: ProtocolRequest,
    pub example_response: ProtocolResponse,
    pub sample_analysis: Option<MidiAnalysisData>,
    pub latest_analysis_job: Option<MidiAnalysisJobStatus>,
    pub latest_process_job: MidiProcessStatus,
    pub latest_process_event: Option<MidiProcessEvent>,
    pub active_process_job_id: Option<MidiProcessJobId>,
    pub latest_audio_render_job: AudioRenderStatus,
    pub latest_audio_render_event: Option<AudioRenderEvent>,
}

impl Default for ProtocolSchemaDemo {
    fn default() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            example_request: ProtocolRequest::default(),
            example_response: ProtocolResponse {
                protocol_version: PROTOCOL_VERSION,
                id: None,
                events: Vec::new(),
            },
            sample_analysis: None,
            latest_analysis_job: None,
            latest_process_job: MidiProcessStatus::Idle,
            latest_process_event: None,
            active_process_job_id: None,
            latest_audio_render_job: AudioRenderStatus::Idle,
            latest_audio_render_event: None,
        }
    }
}
