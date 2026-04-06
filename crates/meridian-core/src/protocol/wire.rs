use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    audio::AudioRenderEvent,
    midi::{MidiFileProcessingConfig, MidiFilesMergeConfig},
    protocol::{
        AnalysisJobId, AudioRenderStatus, CoreCommand, CoreErrorCode, CoreEvent, DisplayCacheId,
        ImageExportConfig, MidiAnalysisData, MidiAnalysisJobEvent, MidiAnalysisJobStatus,
        MidiAnalysisKind, MidiFileInspection, MidiProcessEvent, MidiProcessJobId,
        MidiProcessStatus, PROTOCOL_VERSION, ParsedMidiId, VideoExportConfig, VideoRenderEvent,
        VideoRenderStatus,
    },
    render::{DisplayTimeSpace, RendererKind, SceneConfig, SceneLayout},
};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct ProtocolAudioRenderConfig {
    pub midi_path: PathBuf,
    pub output: PathBuf,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub use_limiter: Option<bool>,
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
    pub first_key: Option<u8>,
    pub last_key: Option<u8>,
    #[serde(default)]
    pub ffmpeg_args: Vec<String>,
    #[serde(default)]
    pub export: VideoExportConfig,
}

impl From<ProtocolAudioRenderConfig> for crate::audio::AudioRenderConfig {
    fn from(value: ProtocolAudioRenderConfig) -> Self {
        Self {
            midi_path: Some(value.midi_path),
            audio: None,
            output: value.output,
            sample_rate: value.sample_rate,
            channels: value.channels,
            use_limiter: value.use_limiter,
            soundfonts: value.soundfonts,
        }
    }
}

impl From<ProtocolVideoRenderConfig> for crate::protocol::VideoRenderConfig {
    fn from(value: ProtocolVideoRenderConfig) -> Self {
        let scene = value.scene.unwrap_or_else(|| {
            let mut layout = SceneLayout::default();
            if let Some(renderer) = value.renderer {
                layout.set_renderer_kind(renderer);
            }
            layout.scene
        });
        Self {
            midi_path: Some(value.midi_path),
            output: value.output,
            fps: value.fps,
            width: value.width,
            height: value.height,
            scene: Some(scene),
            view_range: value.view_range,
            time_space: value.time_space,
            first_key: value.first_key,
            last_key: value.last_key,
            ffmpeg_args: value.ffmpeg_args,
            export: value.export,
        }
    }
}

impl From<crate::protocol::StateSnapshot> for ProtocolStateSnapshot {
    fn from(value: crate::protocol::StateSnapshot) -> Self {
        Self {
            midi_path: value.midi_path,
            current_time: value.current_time,
            playing: value.playing,
            scene: value.scene,
            view_range: value.view_range,
            time_space: value.time_space,
            first_key: value.first_key,
            last_key: value.last_key,
            viewport_width: value.viewport_width,
            viewport_height: value.viewport_height,
        }
    }
}

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
        display_cache_id: Option<DisplayCacheId>,
        #[serde(default)]
        kinds: Vec<MidiAnalysisKind>,
        #[serde(default)]
        bucket_count: Option<usize>,
    },
    GetMidiAnalysisJobStatus {
        job_id: AnalysisJobId,
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

impl From<ProtocolCommand> for CoreCommand {
    fn from(value: ProtocolCommand) -> Self {
        match value {
            ProtocolCommand::LoadParsedMidi { path } => Self::LoadParsedMidi { path },
            ProtocolCommand::InspectMidiFiles { paths } => Self::InspectMidiFiles { paths },
            ProtocolCommand::LoadMidi { path } => Self::LoadMidi { path },
            ProtocolCommand::LoadAudioMidi { path } => Self::LoadAudioMidi { path },
            ProtocolCommand::StartMidiAnalysisJob {
                parsed_midi_id,
                display_cache_id,
                kinds,
                bucket_count,
            } => Self::StartMidiAnalysisJob {
                parsed_midi_id,
                display_cache_id,
                kinds,
                bucket_count,
            },
            ProtocolCommand::GetMidiAnalysisJobStatus { job_id } => {
                Self::GetMidiAnalysisJobStatus { job_id }
            }
            ProtocolCommand::StartProcessMidiFile {
                input,
                output,
                config,
            } => Self::StartProcessMidiFile {
                input,
                output,
                config,
            },
            ProtocolCommand::MergeMidiFiles {
                inputs,
                output,
                config,
            } => Self::MergeMidiFiles {
                inputs,
                output,
                config,
            },
            ProtocolCommand::CancelMidiFileProcess => Self::CancelMidiFileProcess,
            ProtocolCommand::GetMidiFileProcessStatus => Self::GetMidiFileProcessStatus,
            ProtocolCommand::StartRenderAudio { config } => Self::StartRenderAudio {
                config: config.into(),
            },
            ProtocolCommand::CancelRenderAudio => Self::CancelRenderAudio,
            ProtocolCommand::GetRenderAudioStatus => Self::GetRenderAudioStatus,
            ProtocolCommand::SetTime { time } => Self::SetTime { time },
            ProtocolCommand::TickProjectorPhysics { delta_seconds } => {
                Self::TickProjectorPhysics { delta_seconds }
            }
            ProtocolCommand::ResetProjectorPhysics => Self::ResetProjectorPhysics,
            ProtocolCommand::StepTime { delta } => Self::StepTime { delta },
            ProtocolCommand::SetPlaying { playing } => Self::SetPlaying { playing },
            ProtocolCommand::TogglePlaying => Self::TogglePlaying,
            ProtocolCommand::SetSceneConfig { scene } => Self::SetSceneConfig { scene },
            ProtocolCommand::SetViewRange {
                seconds,
                time_space,
            } => Self::SetViewRange {
                seconds,
                time_space,
            },
            ProtocolCommand::SetKeyRange {
                first_key,
                last_key,
            } => Self::SetKeyRange {
                first_key,
                last_key,
            },
            ProtocolCommand::SetViewport { width, height } => Self::SetViewport { width, height },
            ProtocolCommand::SaveFrame {
                output,
                format,
                viewport_width,
                viewport_height,
                export,
            } => Self::SaveFrame {
                output,
                format,
                viewport_width,
                viewport_height,
                export,
            },
            ProtocolCommand::StartRenderVideo { config } => Self::StartRenderVideo {
                config: config.into(),
            },
            ProtocolCommand::CancelRenderVideo => Self::CancelRenderVideo,
            ProtocolCommand::GetRenderVideoStatus => Self::GetRenderVideoStatus,
            ProtocolCommand::Shutdown => Self::Shutdown,
        }
    }
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

impl TryFrom<CoreEvent> for ProtocolEvent {
    type Error = UnsupportedProtocolEvent;

    fn try_from(value: CoreEvent) -> Result<Self, UnsupportedProtocolEvent> {
        match value {
            CoreEvent::ParsedMidiLoaded {
                parsed_midi_id,
                path,
            } => Ok(Self::ParsedMidiLoaded {
                parsed_midi_id,
                path,
            }),
            CoreEvent::MidiFilesInspected { inspections } => {
                Ok(Self::MidiFilesInspected { inspections })
            }
            CoreEvent::StateSnapshot { state } => Ok(Self::StateSnapshot {
                state: state.into(),
            }),
            CoreEvent::MidiLoaded { path, .. } => Ok(Self::MidiLoaded { path }),
            CoreEvent::MidiFileProcessed {
                input,
                output,
                output_track_count,
                output_ppq,
                total_events,
            } => Ok(Self::MidiFileProcessed {
                input,
                output,
                output_track_count,
                output_ppq,
                total_events,
            }),
            CoreEvent::MidiFilesMerged {
                output,
                input_count,
                output_track_count,
                output_ppq,
                total_events,
            } => Ok(Self::MidiFilesMerged {
                output,
                input_count,
                output_track_count,
                output_ppq,
                total_events,
            }),
            CoreEvent::MidiAnalysisJob { event } => Ok(Self::MidiAnalysisJob { event }),
            CoreEvent::MidiAnalysisJobStatus { status } => {
                Ok(Self::MidiAnalysisJobStatus { status })
            }
            CoreEvent::MidiProcess { event } => Ok(Self::MidiProcess { event }),
            CoreEvent::MidiProcessStatus { status } => Ok(Self::MidiProcessStatus { status }),
            CoreEvent::AudioRender { event } => Ok(Self::AudioRender { event }),
            CoreEvent::AudioRenderStatus { status } => Ok(Self::AudioRenderStatus { status }),
            CoreEvent::FrameSaved {
                output,
                format,
                stats,
                bytes_written,
                exports,
                ..
            } => Ok(Self::FrameSaved {
                output,
                format,
                stats,
                bytes_written,
                exports,
            }),
            CoreEvent::VideoRender { event } => Ok(Self::VideoRender { event }),
            CoreEvent::VideoRenderStatus { status } => Ok(Self::VideoRenderStatus { status }),
            CoreEvent::Error { code, message } => Ok(Self::Error { code, message }),
            CoreEvent::ShutdownComplete => Ok(Self::ShutdownComplete),
            _ => Err(UnsupportedProtocolEvent),
        }
    }
}

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
