use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::audio::{AudioRenderEvent, AudioStatus};
use crate::midi::analysis::{MidiAnalysisData, MidiFileInspection};
use crate::render::SceneLayout;

use super::{
    analysis_jobs::{MidiAnalysisJobEvent, MidiAnalysisJobStatus},
    commands::ImageOutputFormat,
    ids::{
        AudioCacheId, AudioSessionId, DisplayCacheId, DisplaySessionId, ParsedMidiId,
        ProcessedMidiId,
    },
    process::{MidiProcessEvent, MidiProcessStatus},
    render::{AudioRenderStatus, FrameStats, VideoRenderEvent, VideoRenderStatus},
    state::StateSnapshot,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CoreErrorCode {
    InvalidJson,
    InvalidProtocolVersion,
    InvalidCommand,
    InvalidRequest,
    InvalidState,
    ValidationFailed,
    InvalidViewport,
    InvalidLayout,
    NoMidiLoaded,
    ResourceNotFound,
    Conflict,
    UnsupportedFormat,
    Unsupported,
    Io,
    Transport,
    Backend,
    ExternalTool,
    Cancelled,
    Internal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CoreEvent {
    ParsedMidiLoaded {
        parsed_midi_id: ParsedMidiId,
        path: PathBuf,
    },
    MidiFilesInspected {
        inspections: Vec<MidiFileInspection>,
    },
    MidiAnalysis {
        processed_midi_id: Option<ProcessedMidiId>,
        display_cache_id: Option<DisplayCacheId>,
        analysis: MidiAnalysisData,
    },
    MidiAnalysisJob {
        event: MidiAnalysisJobEvent,
    },
    MidiAnalysisJobStatus {
        status: MidiAnalysisJobStatus,
    },
    ProcessedMidiBuilt {
        parsed_midi_id: ParsedMidiId,
        processed_midi_id: ProcessedMidiId,
        midi_length: f64,
        total_notes: u64,
        total_audio_events: usize,
        track_count: usize,
    },
    MidiFilesProcessed {
        output: PathBuf,
        input_count: usize,
        output_track_count: usize,
        output_ppq: u16,
        total_events: usize,
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
    DisplaySessionCreated {
        display_session_id: DisplaySessionId,
        display_cache_id: DisplayCacheId,
    },
    AudioSessionCreated {
        audio_session_id: AudioSessionId,
        audio_cache_id: AudioCacheId,
    },
    DisplayCacheAttached {
        display_cache_id: DisplayCacheId,
        state: StateSnapshot,
    },
    ProcessedMidiAttached {
        processed_midi_id: ProcessedMidiId,
        state: StateSnapshot,
    },
    AudioCacheAttached {
        audio_cache_id: AudioCacheId,
        state: StateSnapshot,
    },
    DisplaySessionAttached {
        display_session_id: DisplaySessionId,
        state: StateSnapshot,
    },
    AudioSessionAttached {
        audio_session_id: AudioSessionId,
        state: StateSnapshot,
    },
    StateSnapshot {
        state: StateSnapshot,
    },
    MidiLoadProgress {
        path: PathBuf,
        progress: Option<f32>,
        status: String,
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
    MidiProcess {
        event: MidiProcessEvent,
    },
    MidiProcessStatus {
        status: MidiProcessStatus,
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
