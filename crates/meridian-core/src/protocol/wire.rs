use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    audio::AudioRenderEvent,
    midi::{MidiFileProcessingConfig, MidiFileSelection},
    protocol::{
        AnalysisJobId, AudioRenderStatus, CoreCommand, CoreErrorCode, CoreEvent, DisplayCacheId,
        MidiAnalysisData, MidiAnalysisJobEvent, MidiAnalysisJobStatus, MidiAnalysisKind,
        MidiProcessEvent, MidiProcessJobId, MidiProcessStatus, PROTOCOL_VERSION, ParsedMidiId,
        VideoRenderEvent, VideoRenderStatus,
    },
    render::{DisplayTimeSpace, RendererKind, SceneLayout},
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
pub struct ProtocolVideoRenderConfig {
    pub midi_path: PathBuf,
    pub output: PathBuf,
    pub fps: f64,
    pub width: u32,
    pub height: u32,
    pub renderer: RendererKind,
    pub view_range: Option<f64>,
    #[serde(default)]
    pub time_space: Option<DisplayTimeSpace>,
    pub first_key: Option<u8>,
    pub last_key: Option<u8>,
    #[serde(default)]
    pub ffmpeg_args: Vec<String>,
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
        let mut layout = SceneLayout::default();
        layout.set_renderer_kind(value.renderer);
        Self {
            midi_path: Some(value.midi_path),
            output: value.output,
            fps: value.fps,
            width: value.width,
            height: value.height,
            scene: Some(layout.scene),
            view_range: value.view_range,
            time_space: value.time_space,
            first_key: value.first_key,
            last_key: value.last_key,
            ffmpeg_args: value.ffmpeg_args,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProtocolCommand {
    LoadParsedMidi {
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
    StartProcessMidiFiles {
        selection: MidiFileSelection,
        output: PathBuf,
        config: MidiFileProcessingConfig,
    },
    CancelProcessMidiFiles,
    GetProcessMidiStatus,
    StartRenderAudio {
        config: ProtocolAudioRenderConfig,
    },
    CancelRenderAudio,
    GetRenderAudioStatus,
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
            ProtocolCommand::StartProcessMidiFiles {
                selection,
                output,
                config,
            } => Self::StartProcessMidiFiles {
                selection,
                output,
                config,
            },
            ProtocolCommand::CancelProcessMidiFiles => Self::CancelProcessMidiFiles,
            ProtocolCommand::GetProcessMidiStatus => Self::GetProcessMidiStatus,
            ProtocolCommand::StartRenderAudio { config } => Self::StartRenderAudio {
                config: config.into(),
            },
            ProtocolCommand::CancelRenderAudio => Self::CancelRenderAudio,
            ProtocolCommand::GetRenderAudioStatus => Self::GetRenderAudioStatus,
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
    MidiLoaded {
        path: PathBuf,
    },
    MidiFilesProcessed {
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
            CoreEvent::MidiLoaded { path, .. } => Ok(Self::MidiLoaded { path }),
            CoreEvent::MidiFilesProcessed {
                output,
                input_count,
                output_track_count,
                output_ppq,
                total_events,
            } => Ok(Self::MidiFilesProcessed {
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
            command: ProtocolCommand::GetProcessMidiStatus,
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
