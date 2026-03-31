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
    },
};

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SdkAudioRenderConfig {
    pub midi_path: PathBuf,
    pub output: PathBuf,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub use_limiter: Option<bool>,
    pub soundfonts: Vec<PathBuf>,
}

impl From<SdkAudioRenderConfig> for crate::audio::AudioRenderConfig {
    fn from(value: SdkAudioRenderConfig) -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SdkCommand {
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
        config: SdkAudioRenderConfig,
    },
    CancelRenderAudio,
    GetRenderAudioStatus,
    Shutdown,
}

impl From<SdkCommand> for CoreCommand {
    fn from(value: SdkCommand) -> Self {
        match value {
            SdkCommand::LoadParsedMidi { path } => Self::LoadParsedMidi { path },
            SdkCommand::LoadAudioMidi { path } => Self::LoadAudioMidi { path },
            SdkCommand::StartMidiAnalysisJob {
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
            SdkCommand::GetMidiAnalysisJobStatus { job_id } => {
                Self::GetMidiAnalysisJobStatus { job_id }
            }
            SdkCommand::StartProcessMidiFiles {
                selection,
                output,
                config,
            } => Self::StartProcessMidiFiles {
                selection,
                output,
                config,
            },
            SdkCommand::CancelProcessMidiFiles => Self::CancelProcessMidiFiles,
            SdkCommand::GetProcessMidiStatus => Self::GetProcessMidiStatus,
            SdkCommand::StartRenderAudio { config } => Self::StartRenderAudio {
                config: config.into(),
            },
            SdkCommand::CancelRenderAudio => Self::CancelRenderAudio,
            SdkCommand::GetRenderAudioStatus => Self::GetRenderAudioStatus,
            SdkCommand::Shutdown => Self::Shutdown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SdkEvent {
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
    Error {
        code: CoreErrorCode,
        message: String,
    },
    ShutdownComplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedSdkEvent;

impl TryFrom<CoreEvent> for SdkEvent {
    type Error = UnsupportedSdkEvent;

    fn try_from(value: CoreEvent) -> Result<Self, UnsupportedSdkEvent> {
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
            CoreEvent::Error { code, message } => Ok(Self::Error { code, message }),
            CoreEvent::ShutdownComplete => Ok(Self::ShutdownComplete),
            _ => Err(UnsupportedSdkEvent),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SdkJsonRequest {
    pub protocol_version: u32,
    pub id: Option<u64>,
    pub command: SdkCommand,
}

impl Default for SdkJsonRequest {
    fn default() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            id: None,
            command: SdkCommand::GetProcessMidiStatus,
        }
    }
}

impl From<SdkJsonRequest> for crate::protocol::JsonRequest {
    fn from(value: SdkJsonRequest) -> Self {
        Self {
            protocol_version: value.protocol_version,
            id: value.id,
            command: value.command.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SdkJsonResponse {
    pub protocol_version: u32,
    pub id: Option<u64>,
    pub events: Vec<SdkEvent>,
}

impl TryFrom<crate::protocol::JsonResponse> for SdkJsonResponse {
    type Error = UnsupportedSdkEvent;

    fn try_from(value: crate::protocol::JsonResponse) -> Result<Self, UnsupportedSdkEvent> {
        let mut events = Vec::with_capacity(value.events.len());
        for event in value.events {
            events.push(event.try_into()?);
        }
        Ok(Self {
            protocol_version: value.protocol_version,
            id: value.id,
            events,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct SdkSchemaDemo {
    pub protocol_version: u32,
    pub example_request: SdkJsonRequest,
    pub example_response: SdkJsonResponse,
    pub sample_analysis: Option<MidiAnalysisData>,
    pub latest_analysis_job: Option<MidiAnalysisJobStatus>,
    pub latest_process_job: MidiProcessStatus,
    pub latest_process_event: Option<MidiProcessEvent>,
    pub active_process_job_id: Option<MidiProcessJobId>,
    pub latest_audio_render_job: AudioRenderStatus,
    pub latest_audio_render_event: Option<AudioRenderEvent>,
}

impl Default for SdkSchemaDemo {
    fn default() -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            example_request: SdkJsonRequest::default(),
            example_response: SdkJsonResponse {
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
